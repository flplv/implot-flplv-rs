use bindgen::{Builder, CargoCallbacks};
use std::{env, io::Write, path::PathBuf};
use lazy_static::lazy_static;
use regex::Regex;

// All this crate does is run bindgen on cimplot and store the result
// in the src folder of the implot-sys crate. We add those bindings
// to git so people don't have to install clang just to use implot-rs.

fn main() {
    let cwd = env::current_dir().expect("Could not read current directory");
    let sys_crate_path = cwd
        .join("..")
        .join("implot-sys")
        .canonicalize()
        .expect("Could not find sys crate directory");

    let cimgui_include_path = PathBuf::from(
        env::var_os("DEP_IMGUI_THIRD_PARTY").expect("DEP_IMGUI_THIRD_PARTY not defined"),
    );

    let builder = Builder::default()
        .header(
            cimgui_include_path
                .join("cimgui.h")
                .to_str()
                .expect("Could not convert cimgui.h path to string"),
        )
        .header(
            sys_crate_path
                .join("third-party")
                .join("cimplot")
                .join("cimplot.h")
                .to_str()
                .expect("Could not turn cimplot.h path into string"),
        )
        .parse_callbacks(Box::new(CargoCallbacks::new()))
        .clang_arg("-DCIMGUI_DEFINE_ENUMS_AND_STRUCTS=1")
        .clang_arg(
            String::from("-I")
                + cimgui_include_path
                    .to_str()
                    .expect("Could not turn cimplot/ path into string"),
        )
        // Reuse the imgui types that implot requires from imgui_sys so we don't define
        // our own new types.
        .raw_line("pub use imgui_sys::{ImVec2, ImVec4, ImRect, ImVector_float, ImVector_int};")
        .raw_line("pub use imgui_sys::{ImVector_ImGuiColorMod, ImVector_ImGuiStyleMod};")
        .raw_line("pub use imgui_sys::{ImVector_ImGuiTable, ImVector_ImGuiTabBar};")
        .raw_line("pub use imgui_sys::{ImGuiCond, ImTextureID, ImGuiID};")
        .raw_line("pub use imgui_sys::{ImGuiContext, ImGuiTextBuffer, ImDrawList, ImGuiStorage};")
        .raw_line("pub use imgui_sys::{ImGuiMouseButton, ImGuiDragDropFlags};")
        .raw_line("pub use libc::tm;")
        .raw_line("pub use libc::time_t;")
        .allowlist_recursively(false)
        .allowlist_function("ImPlot.*")
        .allowlist_type("ImPlot.*")
        .allowlist_function("ImPool.*")
        .allowlist_type("ImPool.*")
        // We do want to create bindings for the scalar typedefs
        .allowlist_type("Im[U|S][0-9]{1,2}")
        // And some vectors too
        .allowlist_type("ImAxis.*")
        .allowlist_function("ImVector_ImPlot.*")
        .allowlist_type("ImVector_ImPlot.*")
        .allowlist_type("ImVector_Im[U|S][0-9]{1,2}")
        .allowlist_type("ImVector_double")
        .allowlist_type("ImVector_bool")
        // Remove some functions that would take a variable-argument list
        .blocklist_function("ImPlot_TagX_Str")
        .blocklist_function("ImPlot_TagY_Str")
        .blocklist_function("ImPlot_TagXV")
        .blocklist_function("ImPlot_TagYV")
        .blocklist_function("ImPlot_AnnotationV")
        .blocklist_function("ImPlotTagCollection_Append")
        .blocklist_function("ImPlotTagCollection_AppendV")
        .blocklist_function("ImPlotAnnotationCollection_Append")
        .blocklist_function("ImPlotAnnotationCollection_AppendV")
        // .opaque_type("ImPlotPoint")
        // .opaque_type("ImPlotRange")
        // .opaque_type("ImPlotRect")
        // .opaque_type("ImPlotInputMap")
        // .opaque_type("ImPlotContext")
        // .opaque_type("ImPlotPointError")
        // .opaque_type("ImPlotAnnotation")
        // .opaque_type("ImPlotAnnotationCollection")
        // .opaque_type("ImPlotTagCollection")
        // .opaque_type("ImPlotTick")
        // .opaque_type("ImPlotTicker")
        // .opaque_type("ImPlotAxis")
        // .opaque_type("ImPlotAlignmentData")
        // .opaque_type("ImPlotItem")
        // .opaque_type("ImPlotLegend")
        // .opaque_type("ImPlotItemGroup")
        // .opaque_type("ImPlotPlot")
        // .opaque_type("ImPlotSubplot")
        // .opaque_type("ImPlotNextPlotData")
        // .opaque_type("ImPlotNextItemData")
        .prepend_enum_name(false)
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: (true),
        });


    println!("{}", builder.command_line_flags().join(" ").to_string());

    let bindings = builder.generate().expect("Unable to generate bindings");

    // The above type re-export shenanigans make bindgen unable to derive Copy, Clone and Debug on
    // some types, but they would work - we hence manually re-add them here.
    let mut bindings_string = bindings.to_string();
    ["ImPlotInputMap", "ImPlotStyle"].iter().for_each(|name| {
        bindings_string = bindings_string.replace(
            &format!("pub struct {}", name),
            &format!("#[derive(Clone, Copy, Debug)]\npub struct {}", name),
        );
    });

    // Let's remove the overlyu verbose naming in enums
    lazy_static! {
        static ref RE: Regex = Regex::new(r"pub enum (\S+) \{").unwrap();
    }
    RE.captures_iter(&bindings_string.clone()).map(|c|c.extract()).for_each(|(_, [name])| {
        bindings_string = clear_enum(&bindings_string, name);
    });

    // // Now we create the rustified layer
    // let raiis = ["ImPlotPoint", "ImPlotRange"];
    // let output : String;
    
    // output = generate_raiis(&bindings_string, raiis); 

    // We write the bindings to a file.
    let out_path = sys_crate_path.join("src");
    let mut out_file =
        std::fs::File::create(&out_path.join("bindings.rs")).expect("Could not open bindings file");
    out_file
        .write_all(&bindings_string.into_bytes()[..])
        .expect("Couldn't write bindings");
}

fn clear_enum(bindings_string: &String, name: &str) -> String {
    let title = &format!("pub enum {}", name);
    let begin = bindings_string
        .find(title)
        .expect(&format!("Bad enum name: {}", name)) + title.len();

    let end = bindings_string[begin..]
        .find("}").map(|i| i + begin)
        .expect(&format!("Bad enum formation, no '}}': {}", name));

    let new_enum_body = bindings_string[begin..end].replace(
        name,
        ""
    );

    return bindings_string[..begin].to_owned() + &new_enum_body + &bindings_string[end..];
}

// TODO(lavratti)
// fn generate_raiis<const N: usize>(bindings_string: &String, raiis: [&str;N]) -> String {
//     let mut result = String::new();
//     for raii in raiis {
//         let mut def_constructor = false;
//         let mut constructors: Vec<String> = Vec::new();
//         let mut methods: Vec<&str> = Vec::new();
//         let mut def_destructor = false;

//         let re: Regex = Regex::new(&format!(r"pub fn {}_(\S+)\((.*)\) ?[->]* ?(.*);", raii)).unwrap();
//         re.captures_iter(&bindings_string).map(|c|c.extract()).for_each(|(_, [func, sig, ret])| {
//             if func == raii {
//                 def_constructor = true;
//             } else if func.starts_with(raii) {
//                 constructors.push(format!("
//                     fn new_with_{}({}) -> {} {{

//                     }}
//                 ", 
//                 lowercase_first_letter(&func[raii.len()+1..]),
//                 sig,
//                 raii,

//             ));
//             } else if func == "destroy" {
//                 def_destructor = true;
//             } else {
//                 methods.push(&func);
//             }
//         });
//         result.push_str(format!("impl {} {{\n", raii).as_str());
//         constructors.iter().for_each(|c| {
//             result.push_str(
//                 format!("fn new_from_{} {{,
//                 ",
//                  raii).as_str());
//         });
//         result.push_str("}\n");
//     }
//     result
// }

// fn lowercase_first_letter(s: &str) -> String {
//     let mut c = s.chars();
//     match c.next() {
//         None => String::new(),
//         Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
//     }
// }