fn main() {
    println!("cargo:rerun-if-changed=html/index.html");
    println!("cargo:rerun-if-changed=html/photo.html");
}
