pub mod actor;
pub mod cg;
pub mod colors;
pub mod discord;
pub mod id;

pub fn format(format: &str, args: &[&str]) -> String {
    let mut string = format.to_string();
    for arg in args {
        string = string.replacen("{}", arg, 1);
    }
    string
}
