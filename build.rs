// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set("ProductName", "rbxyoinker")
        .set("FileDescription", "rbxyoinker - Roblox asset downloader")
        .set("CompanyName", "3vil17")
        .set("LegalCopyright", "Copyright (c) 2026 3vil17. AGPL-3.0 License.")
        .set("OriginalFilename", "rbxyoinker.exe")
        .set("Comments", "https://github.com/3vil17/rbxyoinker");
    if let Err(e) = res.compile() {
        println!("cargo:warning=version info not embedded: {e}");
    }
}
