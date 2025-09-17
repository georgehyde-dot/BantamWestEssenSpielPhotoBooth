use actix_web::{get, HttpResponse, Responder};

// Embed HTML files at compile time
const WEAPON_HTML: &str = include_str!("../../html/weapon.html");
const LAND_HTML: &str = include_str!("../../html/land.html");
const COMPANION_HTML: &str = include_str!("../../html/companion.html");
const WANTED_POSTER_WALL_HTML: &str = include_str!("../../html/wanted_poster_wall.html");

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

#[get("/wanted_poster_wall")]
pub async fn wanted_poster_wall_page() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(WANTED_POSTER_WALL_HTML)
}
