#[cfg(not(target_arch = "wasm32"))]
mod native {
    extern crate bord;

    use actix_web::{web, App, HttpServer, HttpRequest, HttpResponse};

    mod adapter {
        use actix_web::HttpRequest;
        use spin_sdk::http::{Request, Response, Method};

        pub fn actix_to_spin_request(
            req: &HttpRequest,
            body: actix_web::web::Bytes,
        ) -> anyhow::Result<Request> {
            let method = match req.method().as_str() {
                "GET" => Method::Get,
                "POST" => Method::Post,
                "PUT" => Method::Put,
                "DELETE" => Method::Delete,
                "HEAD" => Method::Head,
                "OPTIONS" => Method::Options,
                "PATCH" => Method::Patch,
                _ => Method::Get,
            };

            let uri = req.uri().to_string();
            let body_vec = body.to_vec();
            
            let mut req_builder = Request::builder();
            let method_set = req_builder.method(method);
            let uri_set = method_set.uri(&uri);

            // Copy headers
            let mut with_headers = uri_set;
            for (name, value) in req.headers() {
                if let Ok(val_str) = value.to_str() {
                    with_headers = with_headers.header(name.as_str(), val_str);
                }
            }

            Ok(with_headers.body(body_vec).build())
        }

        pub fn spin_to_actix_response(spin_resp: spin_sdk::http::Response) -> actix_web::HttpResponse {
            let status = *spin_resp.status();
            let body = spin_resp.body().to_vec();

            let mut response = actix_web::HttpResponse::build(
                actix_web::http::StatusCode::from_u16(status)
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
            );

            response.body(body)
        }
    }

    pub async fn run() -> std::io::Result<()> {
        println!("Server listening on http://0.0.0.0:80");

        HttpServer::new(|| {
            App::new()
                .default_service(web::route().to(handle_all))
        })
        .bind("0.0.0.0:80")?
        .run()
        .await
    }

    async fn handle_all(req: HttpRequest, body: web::Bytes) -> HttpResponse {
        let path = req.path().to_string();
        let method = req.method().as_str();

        let spin_req = match adapter::actix_to_spin_request(&req, body) {
            Ok(r) => r,
            Err(_) => {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Invalid request"}))
            }
        };

        let result = match (method, path.as_str()) {
            ("POST", "/users") => bord::users::create_user(spin_req),
            ("POST", "/login") => bord::auth::login_user(spin_req),
            ("POST", "/logout") => bord::auth::logout_user(spin_req),
            ("GET", "/profile") => bord::users::get_profile(spin_req),
            ("PUT", "/profile") => bord::users::update_profile(spin_req),
            ("POST", "/posts") => bord::posts::create_post(spin_req),
            ("GET", "/posts") => bord::posts::list_posts(spin_req),
            ("GET", "/feed") => bord::posts::get_feed(spin_req),
            ("POST", "/follow") => bord::follow::handle_follow(spin_req),
            ("POST", "/unfollow") => bord::follow::handle_unfollow(spin_req),
            ("PUT", p) if p.starts_with("/posts/") => bord::posts::edit_post(spin_req),
            ("DELETE", p) if p.starts_with("/posts/") => bord::posts::delete_post(spin_req),
            ("GET", p) if p.starts_with("/followings/") => bord::follow::get_followings_list(p),
            ("GET", p) if p.starts_with("/followers/") => bord::follow::get_followers_list(p),
            ("GET", p) if p.starts_with("/users/") && p.len() > 7 => bord::users::get_user_details(p),
            ("GET", p) if !p.contains('.') && p.len() > 1 && p != "/" => {
                bord::templates::render_user_profile(&spin_req, p)
            }
            ("GET", p) => bord::core::static_server::serve_static(p),
            _ => {
                return HttpResponse::NotFound()
                    .json(serde_json::json!({"error": "No route found"}))
            }
        };

        match result {
            Ok(spin_resp) => adapter::spin_to_actix_response(spin_resp),
            Err(_) => HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Internal server error"})),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    native::run().await
}

#[cfg(target_arch = "wasm32")]
fn main() {}
