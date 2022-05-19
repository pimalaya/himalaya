extern crate bindgen;
extern crate cc;
extern crate embed_resource;
extern crate pkg_config;

use bindgen::Builder as BindgenBuilder;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Deterimine build platform
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_triple = env::var("TARGET").unwrap();
    let msvc = target_triple.contains("msvc");
    let apple = target_triple.contains("apple");
    let unix = cfg!(target_family = "unix") && !apple;

    // Fetch the submodule if needed
    if cfg!(feature = "fetch") {
        // Init or update the submodule with libui if needed
        if !Path::new("libui/.git").exists() {
            Command::new("git")
                .args(&["version"])
                .status()
                .expect("Git does not appear to be installed. Error");
            Command::new("git")
                .args(&["submodule", "update", "--init"])
                .status()
                .expect("Unable to init libui submodule. Error");
        } else {
            Command::new("git")
                .args(&["submodule", "update", "--recursive"])
                .status()
                .expect("Unable to update libui submodule. Error");
        }
    }

    // Generate libui bindings on the fly
    let bindings = BindgenBuilder::default()
        .header("wrapper.h")
        .opaque_type("max_align_t") // For some reason this ends up too large
        //.rustified_enum(".*")
        .trust_clang_mangling(false) // clang sometimes wants to treat these functions as C++
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings");

    // Build libui if needed. Otherwise, assume it's in lib/
    if cfg!(feature = "build") {
        let mut base_config = cc::Build::new();
        let src_base = env::var("SRC_BASE").unwrap_or("libui".to_string());
        let src_path = |x| format!("{}/{}", src_base, x);

        // Add source files that are common to all platforms
        base_config.include(src_path("/common"));

        for filename in [
            "common/attribute.c",
            "common/attrlist.c",
            "common/attrstr.c",
            "common/areaevents.c",
            "common/control.c",
            "common/debug.c",
            "common/matrix.c",
            "common/opentype.c",
            "common/shouldquit.c",
            "common/tablemodel.c",
            "common/tablevalue.c",
            "common/userbugs.c",
            "common/utf.c",
        ]
        .iter()
        {
            base_config.file(src_path(filename));
        }

        if target_os == "windows" {
            base_config.cpp(true);
            base_config.include(src_path("/windows"));

            for filename in [
                "windows/alloc.cpp",
                "windows/area.cpp",
                "windows/areadraw.cpp",
                "windows/areaevents.cpp",
                "windows/areascroll.cpp",
                "windows/areautil.cpp",
                "windows/attrstr.cpp",
                "windows/box.cpp",
                "windows/button.cpp",
                "windows/checkbox.cpp",
                "windows/colorbutton.cpp",
                "windows/colordialog.cpp",
                "windows/combobox.cpp",
                "windows/container.cpp",
                "windows/control.cpp",
                "windows/d2dscratch.cpp",
                "windows/datetimepicker.cpp",
                "windows/debug.cpp",
                "windows/draw.cpp",
                "windows/drawmatrix.cpp",
                "windows/drawpath.cpp",
                "windows/drawtext.cpp",
                "windows/dwrite.cpp",
                "windows/editablecombo.cpp",
                "windows/entry.cpp",
                "windows/events.cpp",
                "windows/fontbutton.cpp",
                "windows/fontdialog.cpp",
                "windows/fontmatch.cpp",
                "windows/form.cpp",
                "windows/graphemes.cpp",
                "windows/grid.cpp",
                "windows/group.cpp",
                "windows/image.cpp",
                "windows/init.cpp",
                "windows/label.cpp",
                "windows/main.cpp",
                "windows/menu.cpp",
                "windows/multilineentry.cpp",
                "windows/opentype.cpp",
                "windows/parent.cpp",
                "windows/progressbar.cpp",
                "windows/radiobuttons.cpp",
                "windows/separator.cpp",
                "windows/sizing.cpp",
                "windows/slider.cpp",
                "windows/spinbox.cpp",
                "windows/stddialogs.cpp",
                "windows/tab.cpp",
                "windows/table.cpp",
                "windows/tabledispinfo.cpp",
                "windows/tabledraw.cpp",
                "windows/tableediting.cpp",
                "windows/tablemetrics.cpp",
                "windows/tabpage.cpp",
                "windows/text.cpp",
                "windows/utf16.cpp",
                "windows/utilwin.cpp",
                "windows/window.cpp",
                "windows/winpublic.cpp",
                "windows/winutil.cpp",
            ]
            .iter()
            {
                base_config.file(src_path(filename));
            }

            // See https://github.com/nabijaczleweli/rust-embed-resource/issues/11
            let target = env::var("TARGET").unwrap();
            if let Some(tool) = cc::windows_registry::find_tool(target.as_str(), "cl.exe") {
                for (key, value) in tool.env() {
                    env::set_var(key, value);
                }
            }
            embed_resource::compile(src_path("/windows/resources.rc"));

            link("user32", false);
            link("kernel32", false);
            link("gdi32", false);
            link("comctl32", false);
            link("uxtheme", false);
            link("msimg32", false);
            link("comdlg32", false);
            link("d2d1", false);
            link("dwrite", false);
            link("ole32", false);
            link("oleaut32", false);
            link("oleacc", false);
            link("uuid", false);
            link("windowscodecs", false);
        } else if unix {
            base_config.include(src_path("/unix"));

            let pkg_cfg = pkg_config::Config::new().probe("gtk+-3.0").unwrap();
            for inc in pkg_cfg.include_paths {
                base_config.include(inc);
            }

            for filename in [
                "unix/alloc.c",
                "unix/area.c",
                "unix/attrstr.c",
                "unix/box.c",
                "unix/button.c",
                "unix/cellrendererbutton.c",
                "unix/checkbox.c",
                "unix/child.c",
                "unix/colorbutton.c",
                "unix/combobox.c",
                "unix/control.c",
                "unix/datetimepicker.c",
                "unix/debug.c",
                "unix/draw.c",
                "unix/drawmatrix.c",
                "unix/drawpath.c",
                "unix/drawtext.c",
                "unix/editablecombo.c",
                "unix/entry.c",
                "unix/fontbutton.c",
                "unix/fontmatch.c",
                "unix/form.c",
                "unix/future.c",
                "unix/graphemes.c",
                "unix/grid.c",
                "unix/group.c",
                "unix/image.c",
                "unix/label.c",
                "unix/main.c",
                "unix/menu.c",
                "unix/multilineentry.c",
                "unix/opentype.c",
                "unix/progressbar.c",
                "unix/radiobuttons.c",
                "unix/separator.c",
                "unix/slider.c",
                "unix/spinbox.c",
                "unix/stddialogs.c",
                "unix/tab.c",
                "unix/table.c",
                "unix/tablemodel.c",
                "unix/text.c",
                "unix/util.c",
                "unix/window.c",
            ]
            .iter()
            {
                base_config.file(src_path(filename));
            }
        } else if apple {
            base_config.include(src_path("/darwin"));

            for filename in [
                "darwin/aat.m",
                "darwin/alloc.m",
                "darwin/area.m",
                "darwin/areaevents.m",
                "darwin/attrstr.m",
                "darwin/autolayout.m",
                "darwin/box.m",
                "darwin/button.m",
                "darwin/checkbox.m",
                "darwin/colorbutton.m",
                "darwin/combobox.m",
                "darwin/control.m",
                "darwin/datetimepicker.m",
                "darwin/debug.m",
                "darwin/draw.m",
                "darwin/drawtext.m",
                "darwin/editablecombo.m",
                "darwin/entry.m",
                "darwin/fontbutton.m",
                "darwin/fontmatch.m",
                "darwin/fonttraits.m",
                "darwin/fontvariation.m",
                "darwin/form.m",
                "darwin/future.m",
                "darwin/graphemes.m",
                "darwin/grid.m",
                "darwin/group.m",
                "darwin/image.m",
                "darwin/label.m",
                "darwin/main.m",
                "darwin/map.m",
                "darwin/menu.m",
                "darwin/multilineentry.m",
                "darwin/opentype.m",
                "darwin/progressbar.m",
                "darwin/radiobuttons.m",
                "darwin/scrollview.m",
                "darwin/separator.m",
                "darwin/slider.m",
                "darwin/spinbox.m",
                "darwin/stddialogs.m",
                "darwin/tab.m",
                "darwin/table.m",
                "darwin/tablecolumn.m",
                "darwin/text.m",
                "darwin/undocumented.m",
                "darwin/util.m",
                "darwin/window.m",
                "darwin/winmoveresize.m",
            ]
            .iter()
            {
                base_config.file(src_path(filename));
            }
            println!("cargo:rustc-link-lib=framework=AppKit");
        } else {
            panic!("unrecognized platform! cannot build libui from source");
        }

        // Link everything together into `libui.a`.  This will get linked
        // together because of the `links="ui"` flag in the `Cargo.toml` file,
        // and because the `.compile()` function emits
        // `cargo:rustc-link-lib=static=ui`.
        base_config.compile("libui.a");
    } else {
        // If we're not building the library, then assume it's pre-built and
        // exists in `lib/`
        let mut dst = env::current_dir().expect("Unable to retrieve current directory location.");
        dst.push("lib");

        let libname = if msvc { "libui" } else { "ui" };

        println!("cargo:rustc-link-search=native={}", dst.display());
        println!("cargo:rustc-link-lib={}", libname);
    }
}

/// Tell cargo to link the given library, and optionally to bundle it in.
pub fn link(name: &str, bundled: bool) {
    let target = env::var("TARGET").unwrap();
    let target: Vec<_> = target.split('-').collect();
    if target.get(2) == Some(&"windows") {
        println!("cargo:rustc-link-lib=dylib={}", name);
        if bundled && target.get(3) == Some(&"gnu") {
            let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            println!("cargo:rustc-link-search=native={}/{}", dir, target[0]);
        }
    } else {
        println!("cargo:rustc-link-lib=dylib={}", name);
    }
}

/// Add the given framework to the linker path
pub fn link_framework(name: &str) {
    println!("cargo:rustc-link-lib=framework={}", name);
}
