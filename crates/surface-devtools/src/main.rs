fn main() {
    std::process::exit(surface_devtools::run(
        std::env::args().skip(1).collect(),
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    ));
}
