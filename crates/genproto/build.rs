fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .out_dir("src/gen")
        .compile_protos(
            &["../../proto/user.proto", "../../proto/auth.proto", "../../proto/post.proto", "../../proto/category.proto", "../../proto/comment.proto"], 
            &["../../proto"],           
        )?;
    println!("cargo:rerun-if-changed=../../proto/movie.proto");
    Ok(())
}
