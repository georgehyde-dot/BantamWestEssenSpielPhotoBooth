use actix_web::{get, web, HttpResponse, Responder};

// Embed HTML files at compile time
const START_HTML: &str = include_str!("../../html/start.html");
const ENTER_NAMES_HTML: &str = include_str!("../../html/enter_names.html");
const COPIES_HTML: &str = include_str!("../../html/copies.html");
const INDEX_HTML: &str = include_str!("../../html/index.html");
const PHOTO_HTML: &str = include_str!("../../html/photo.html");
const STREAM_TEST_HTML: &str = include_str!("../../html/test/stream_test.html");

#[get("/")]
pub async fn start_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(START_HTML)
}

#[get("/name-entry")]
pub async fn name_entry_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(ENTER_NAMES_HTML)
}

#[get("/copies")]
pub async fn copies_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(COPIES_HTML)
}

#[get("/camera")]
pub async fn camera_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

#[get("/photo")]
pub async fn photo_page(
    _query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(PHOTO_HTML)
}

#[get("/test/stream")]
pub async fn test_stream() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(STREAM_TEST_HTML)
}
