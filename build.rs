fn main() {
    println!("cargo:rerun-if-changed=html/start.html");
    println!("cargo:rerun-if-changed=html/enter_names.html");
    println!("cargo:rerun-if-changed=html/index.html");
    println!("cargo:rerun-if-changed=html/photo.html");
    println!("cargo:rerun-if-changed=migrations");
}
