//! Supervised private invitation delivery (S19 stage 2).
use super::delivery;
use crate::{
    lifecycle::{Connections, Shutdown, Task},
    mail::Mailer,
};
use std::{path::PathBuf, time::Duration};

/// Runs private outbox delivery until the process requests shutdown.
pub fn task(
    database: PathBuf,
    connections: Connections,
    mailer: Mailer,
    now: impl Fn() -> i64 + Send + Sync + 'static,
) -> Task {
    configured(
        database,
        connections,
        mailer,
        now,
        #[cfg(test)]
        Hooks::default(),
    )
}

fn configured(
    database: PathBuf,
    connections: Connections,
    mailer: Mailer,
    now: impl Fn() -> i64 + Send + Sync + 'static,
    #[cfg(test)] hooks: Hooks,
) -> Task {
    Task::new("invitation-delivery", move |mut shutdown| async move {
        let mut ticks = tokio::time::interval(Duration::from_secs(1));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            #[cfg(test)]
            if let Some(gate) = &hooks.tick {
                gate.pass().await;
            }
            tokio::select! {
                biased;
                () = shutdown.requested() => return,
                _ = ticks.tick() => {}
            }
            tick(
                &database,
                &connections,
                &mailer,
                &now,
                &shutdown,
                #[cfg(test)]
                &hooks,
            )
            .await;
        }
    })
}

async fn tick(
    database: &std::path::Path,
    connections: &Connections,
    mailer: &Mailer,
    now: &(impl Fn() -> i64 + Sync),
    shutdown: &Shutdown,
    #[cfg(test)] hooks: &Hooks,
) {
    if shutdown.is_requested() {
        return;
    }
    let Ok(mut conn) = connections.open(database).await else {
        report(
            "stage=open result=failed".into(),
            #[cfg(test)]
            hooks,
        );
        return;
    };
    if shutdown.is_requested() {
        return;
    }
    if delivery::sweep(&mut conn, now()).await.is_err() {
        report(
            "stage=sweep result=failed".into(),
            #[cfg(test)]
            hooks,
        );
        return;
    }
    if shutdown.is_requested() {
        return;
    }
    let claim = match delivery::claim(&mut conn, now()).await {
        Ok(Some(claim)) => claim,
        Ok(None) => return,
        Err(_) => {
            report(
                "stage=claim result=failed".into(),
                #[cfg(test)]
                hooks,
            );
            return;
        }
    };
    #[cfg(test)]
    if let Some(gate) = &hooks.before_send {
        gate.pass().await;
    }
    // The claim is spent even if this stop suppresses SMTP. A send admitted
    // below runs through its timeout and fenced completion during the drain.
    if shutdown.is_requested() {
        return;
    }
    let sent = mailer.send(&claim).await;
    #[cfg(test)]
    if let Some(gate) = &hooks.after_send {
        gate.pass().await;
    }
    let result = match delivery::complete(
        &mut conn,
        claim.outbox_id,
        claim.attempt,
        sent.completion,
        now(),
    )
    .await
    {
        Ok(true) => "acknowledged",
        Ok(false) => "no-transition",
        Err(_) => "unconfirmed",
    };
    report(
        format!(
            "outbox={} attempt={} stage=complete category={} result={result}",
            claim.outbox_id,
            claim.attempt,
            sent.category.label()
        ),
        #[cfg(test)]
        hooks,
    );
    // Every tick owns one tracked connection, disposed here on success or any
    // earlier failure. Its existing close acknowledgment releases the ticket.
}

fn report(line: String, #[cfg(test)] hooks: &Hooks) {
    let line = format!("reference-dev: invitation-delivery {line}");
    eprintln!("{line}");
    #[cfg(test)]
    if let Some(sink) = &hooks.sink {
        let _ = sink.send(line);
    }
}

#[cfg(test)]
#[derive(Default)]
struct Hooks {
    tick: Option<std::sync::Arc<crate::lifecycle::Gate>>,
    before_send: Option<std::sync::Arc<crate::lifecycle::Gate>>,
    after_send: Option<std::sync::Arc<crate::lifecycle::Gate>>,
    sink: Option<tokio::sync::mpsc::UnboundedSender<String>>,
}

#[cfg(test)]
mod tests;
