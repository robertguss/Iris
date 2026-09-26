export interface paths {
    "/api/memberships/role": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["changeMemberRole"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        ChangeRoleRequest: {
            project_id: string;
            role: components["schemas"]["Role"];
            user_id: string;
        };
        /** @enum {string} */
        Completion: "acknowledged";
        /** @enum {string} */
        Role: "owner" | "editor" | "viewer";
        SuccessData: {
            completion: components["schemas"]["Completion"];
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    changeMemberRole: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChangeRoleRequest"];
            };
        };
        responses: {
            /** @description S16 response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["SuccessData"];
                        /** @enum {string} */
                        kind: "success";
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "http.invalid_request";
                        /** @enum {string} */
                        kind: "refused";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "http.unauthenticated";
                        /** @enum {string} */
                        kind: "refused";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "memberships.forbidden";
                        /** @enum {string} */
                        kind: "rejected";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    } | {
                        /** @enum {string} */
                        code: "http.csrf_refused";
                        /** @enum {string} */
                        kind: "refused";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "memberships.member_not_found";
                        /** @enum {string} */
                        kind: "rejected";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "memberships.last_owner";
                        /** @enum {string} */
                        kind: "rejected";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "iris.internal";
                        /** @enum {string} */
                        kind: "failure";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description S16 response */
            503: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @enum {string} */
                        code: "iris.unavailable";
                        /** @enum {string} */
                        kind: "failure";
                        message: string;
                        /** @enum {string} */
                        operation: "memberships.change_role";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
        };
    };
}
