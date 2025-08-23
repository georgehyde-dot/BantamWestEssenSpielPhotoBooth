use actix_web::{get, web, HttpResponse, Responder};

// Embed HTML files at compile time
const WEAPON_HTML: &str = include_str!("../../html/weapon.html");
const LAND_HTML: &str = include_str!("../../html/land.html");
const COMPANION_HTML: &str = include_str!("../../html/companion.html");

#[get("/weapon")]
pub async fn weapon_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(WEAPON_HTML)
}

#[get("/land")]
pub async fn land_page() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(LAND_HTML)
}

#[get("/companion")]
pub async fn companion_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(COMPANION_HTML)
}
