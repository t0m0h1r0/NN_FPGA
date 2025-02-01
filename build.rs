use pyo3_build_config::resolve_env_var;

fn main() {
    // Pythonライブラリパスの設定
    if let Ok(python_lib_path) = resolve_env_var("PYTHON_SYS_EXECUTABLE") {
        println!("cargo:rustc-link-search=native={}", python_lib_path);
    }

    // プラットフォーム固有の設定
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=dylib=python3");
    }

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=python3");
    }

    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=python3");
    }
}