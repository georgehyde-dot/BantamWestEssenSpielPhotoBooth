use actix_web::{get, HttpResponse, Responder};

// Embed HTML files at compile time
const CLASS_HTML: &str = include_str!("../../html/class.html");
const LAND_HTML: &str = include_str!("../../html/land.html");
const CHOICE_HTML: &str = include_str!("../../html/choice.html");

#[get("/class")]
pub async fn class_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(CLASS_HTML)
}

#[get("/land")]
pub async fn land_page() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(LAND_HTML)
}

#[get("/choice")]
pub async fn choice_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(CHOICE_HTML)
}
