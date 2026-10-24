fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = enem_backend::openapi::openapi();
    println!("{}", document.to_pretty_json()?);
    Ok(())
}
