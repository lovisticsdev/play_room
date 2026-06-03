use play_room_protocol::{
    typescript_constants_module, typescript_schema_module, typescript_types_module,
};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(default_output_dir);

    fs::create_dir_all(&output_dir)?;
    write_file(
        output_dir.join("generated.ts"),
        typescript_constants_module(),
    )?;
    write_file(output_dir.join("schema.ts"), typescript_schema_module())?;
    write_file(
        output_dir.join("generated-types.ts"),
        typescript_types_module(),
    )?;
    Ok(())
}

fn write_file(path: PathBuf, contents: String) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(&path, contents)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("web/src/lib/protocol")
}
