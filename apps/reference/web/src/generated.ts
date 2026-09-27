export interface paths {
    "/api/auth/callback": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["callback"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/session": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get: operations["session_info"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/memberships/remove": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["removeMember"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
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
        ChangeRoleSuccess: {
            completion: components["schemas"]["Completion"];
        };
        /** @enum {string} */
        Completion: "acknowledged";
        /** @enum {string} */
        ErrorCode: "csrf" | "login_failed" | "unauthorized" | "internal";
        LoginInfo: {
            authorization_url: string;
        };
        Problem: {
            code: components["schemas"]["ErrorCode"];
            message: string;
        };
        RemoveMemberRequest: {
            project_id: string;
            user_id: string;
        };
        RemoveMemberSuccess: {
            completion: components["schemas"]["Completion"];
        };
        /** @enum {string} */
        Role: "owner" | "editor" | "viewer";
        SessionInfo: {
            csrf_token: string;
            user_id?: string | null;
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
    callback: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Login complete */
            303: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
        };
    };
    login: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["LoginInfo"];
                };
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
        };
    };
    logout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description This session logged out */
            204: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
        };
    };
    session_info: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["SessionInfo"];
                };
            };
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["Problem"];
                };
            };
        };
    };
    removeMember: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RemoveMemberRequest"];
            };
        };
        responses: {
            /** @description Iris response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["RemoveMemberSuccess"];
                        /** @enum {string} */
                        kind: "success";
                        /** @enum {string} */
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
            /** @description Iris response */
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
                        operation: "memberships.remove_member";
                        request_id: string;
                        /** @enum {integer} */
                        schema_version: 1;
                    };
                };
            };
        };
    };
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
            /** @description Iris response */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["ChangeRoleSuccess"];
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
            /** @description Iris response */
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
            /** @description Iris response */
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
            /** @description Iris response */
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
            /** @description Iris response */
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
            /** @description Iris response */
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
            /** @description Iris response */
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
            /** @description Iris response */
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
