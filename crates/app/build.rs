fn main() {
    winresource::WindowsResource::new()
        .set_icon("assets/app.ico")
        .compile()
        .unwrap();
}
