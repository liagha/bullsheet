#[cfg(windows)]
fn main() -> std::io::Result<()> {
    let mut resource = winres::WindowsResource::new();
    resource.set_icon("assets/bullsheet.ico");
    resource.set("ProductName", "bullsheet");
    resource.set("FileDescription", "bullsheet");
    resource.compile()?;
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let directory = std::env::var("OUT_DIR").map_err(std::io::Error::other)?;
        println!("cargo:rustc-link-arg={directory}/resource.o");
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {}
