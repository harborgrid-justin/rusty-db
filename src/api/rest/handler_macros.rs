// Handler Macros Module
//
// Helper macros to reduce boilerplate in REST API handlers

/// Macro to create a simple GET handler that returns data from a lazy_static store
///
/// # Usage
/// ```ignore
/// simple_get_handler!(
///     get_users,                    // Function name
///     "/api/v1/users",              // Path
///     "users",                      // Tag
///     Vec<UserResponse>,            // Response type
///     USERS_STORE,                  // Store to read from
///     |store| {                     // Transformation closure
///         store.values().cloned().collect()
///     }
/// );
/// ```
#[macro_export]
macro_rules! simple_get_handler {
    (
        $fn_name:ident,
        $path:expr,
        $tag:expr,
        $response_type:ty,
        $store:ident,
        $transform:expr
    ) => {
        #[utoipa::path(
            get,
            path = $path,
            tag = $tag,
            responses(
                (status = 200, description = "Success", body = $response_type),
            )
        )]
        pub async fn $fn_name(
            State(_state): State<Arc<ApiState>>,
        ) -> ApiResult<AxumJson<$response_type>> {
            let store = $store.read();
            let transform_fn: fn(&_) -> $response_type = $transform;
            let result = transform_fn(&*store);
            Ok(AxumJson(result))
        }
    };
}

/// Macro to create a GET handler with path parameter
///
/// # Usage
/// ```ignore
/// get_by_id_handler!(
///     get_user,                     // Function name
///     "/api/v1/users/{id}",         // Path
///     "users",                      // Tag
///     UserResponse,                 // Response type
///     u64,                          // ID type
///     USERS_STORE,                  // Store to read from
///     |store, id| {                 // Lookup closure
///         store.get(&id).cloned()
///     },
///     "User not found"              // Error message
/// );
/// ```
#[macro_export]
macro_rules! get_by_id_handler {
    (
        $fn_name:ident,
        $path:expr,
        $tag:expr,
        $response_type:ty,
        $id_type:ty,
        $store:ident,
        $lookup:expr,
        $error_msg:expr
    ) => {
        #[utoipa::path(
            get,
            path = $path,
            tag = $tag,
            responses(
                (status = 200, description = "Success", body = $response_type),
                (status = 404, description = "Not found", body = ApiError),
            )
        )]
        pub async fn $fn_name(
            State(_state): State<Arc<ApiState>>,
            Path(id): Path<$id_type>,
        ) -> ApiResult<AxumJson<$response_type>> {
            let store = $store.read();
            let lookup_fn: fn(&_, $id_type) -> Option<$response_type> = $lookup;

            lookup_fn(&*store, id)
                .map(|item| AxumJson(item))
                .ok_or_else(|| ApiError::new("NOT_FOUND", $error_msg))
        }
    };
}

/// Macro to create a POST/CREATE handler
///
/// # Usage
/// ```ignore
/// create_handler!(
///     create_user,                  // Function name
///     "/api/v1/users",              // Path
///     "users",                      // Tag
///     CreateUserRequest,            // Request type
///     UserResponse,                 // Response type
///     USERS_STORE,                  // Store to write to
///     NEXT_USER_ID,                 // ID counter
///     |request, id| {               // Create closure
///         UserResponse {
///             id,
///             username: request.username,
///             email: request.email,
///             created_at: SystemTime::now(),
///         }
///     },
///     |store, item| {               // Store closure
///         store.insert(item.id, item.clone());
///         item
///     }
/// );
/// ```
#[macro_export]
macro_rules! create_handler {
    (
        $fn_name:ident,
        $path:expr,
        $tag:expr,
        $request_type:ty,
        $response_type:ty,
        $store:ident,
        $id_counter:ident,
        $create:expr,
        $store_fn:expr
    ) => {
        #[utoipa::path(
            post,
            path = $path,
            tag = $tag,
            request_body = $request_type,
            responses(
                (status = 201, description = "Created", body = $response_type),
                (status = 400, description = "Bad request", body = ApiError),
            )
        )]
        pub async fn $fn_name(
            State(_state): State<Arc<ApiState>>,
            AxumJson(request): AxumJson<$request_type>,
        ) -> ApiResult<(StatusCode, AxumJson<$response_type>)> {
            // Get next ID
            let id = {
                let mut counter = $id_counter.write();
                let id = *counter;
                *counter += 1;
                id
            };

            // Create item
            let create_fn: fn($request_type, _) -> $response_type = $create;
            let item = create_fn(request, id);

            // Store item
            let store_fn: fn(&mut _, $response_type) -> $response_type = $store_fn;
            let stored_item = {
                let mut store = $store.write();
                store_fn(&mut *store, item)
            };

            Ok((StatusCode::CREATED, AxumJson(stored_item)))
        }
    };
}

/// Macro to create a WebSocket upgrade handler
///
/// # Usage
/// ```ignore
/// ws_upgrade_handler!(
///     ws_metrics_stream,            // Function name
///     "/api/v1/ws/metrics",         // Path
///     "websocket",                  // Tag
///     "Metrics streaming",          // Description
///     handle_metrics_websocket      // Handler function
/// );
/// ```
#[macro_export]
macro_rules! ws_upgrade_handler {
    (
        $fn_name:ident,
        $path:expr,
        $tag:expr,
        $description:expr,
        $handler:ident
    ) => {
        #[utoipa::path(
            get,
            path = $path,
            tag = $tag,
            responses(
                (status = 101, description = "WebSocket upgrade successful"),
                (status = 400, description = "Bad request"),
            )
        )]
        #[doc = $description]
        pub async fn $fn_name(
            ws: WebSocketUpgrade,
            State(state): State<Arc<ApiState>>,
        ) -> Response {
            ws.on_upgrade(|socket| $handler(socket, state))
        }
    };
}

/// Macro to create a simple state-reading GET handler
///
/// # Usage
/// ```ignore
/// state_get_handler!(
///     get_metrics,                  // Function name
///     "/api/v1/metrics",            // Path
///     "monitoring",                 // Tag
///     MetricsResponse,              // Response type
///     |state| async move {          // Async closure to get data
///         let metrics = state.metrics.read().await;
///         MetricsResponse {
///             total_requests: metrics.total_requests,
///             // ... more fields
///         }
///     }
/// );
/// ```
#[macro_export]
macro_rules! state_get_handler {
    (
        $fn_name:ident,
        $path:expr,
        $tag:expr,
        $response_type:ty,
        $get_data:expr
    ) => {
        #[utoipa::path(
            get,
            path = $path,
            tag = $tag,
            responses(
                (status = 200, description = "Success", body = $response_type),
            )
        )]
        pub async fn $fn_name(
            State(state): State<Arc<ApiState>>,
        ) -> ApiResult<AxumJson<$response_type>> {
            let get_fn = $get_data;
            let result = get_fn(state).await;
            Ok(AxumJson(result))
        }
    };
}

/// Macro to wrap WebSocket handler setup (reduces 45 instances of identical async stream setup)
///
/// This macro standardizes the WebSocket handler pattern with welcome message and error handling
///
/// # Usage
/// ```ignore
/// impl_websocket_handler!(handle_metrics_websocket, "metrics", |socket, state| {
///     // Custom handler logic here
///     // Send periodic metrics updates
///     let mut interval = interval(Duration::from_secs(1));
///     loop {
///         interval.tick().await;
///         // Send metrics...
///     }
/// });
/// ```
#[macro_export]
macro_rules! impl_websocket_handler {
    ($handler_name:ident, $connection_type:expr, $handler_logic:expr) => {
        async fn $handler_name(mut socket: WebSocket, state: Arc<ApiState>) {
            use axum::extract::ws::Message;
            use serde_json::json;
            use std::time::{SystemTime, UNIX_EPOCH};

            // Send welcome message
            let welcome = json!({
                "type": "welcome",
                "connection_type": $connection_type,
                "message": format!("Connected to {} stream", $connection_type),
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            });

            if let Ok(welcome_json) = serde_json::to_string(&welcome) {
                if socket.send(Message::Text(welcome_json)).await.is_err() {
                    return;
                }
            }

            // Execute custom handler logic
            let handler: fn(WebSocket, Arc<ApiState>) -> _ = $handler_logic;
            handler(socket, state).await
        }
    };
}
