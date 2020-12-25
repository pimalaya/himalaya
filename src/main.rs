mod config;
mod imap;

fn main() {
    let config = config::read_file();
    let sess = imap::login(&config);
    println!("{:?}", sess);
}
