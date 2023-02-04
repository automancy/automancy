pub mod cg;
pub mod colors;
pub mod discord;
pub mod id;

pub fn format(format: &String, args: Vec<&str>) -> String {
    let mut string = format.clone();
    for arg in args {
        string = string.replacen("{}", arg, 1);
    }
    string
}
