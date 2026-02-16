fn main() {
    match kdotool::get_active_window_info() {
        Ok(info) => {
            println!("id: {}", info.id);
            println!("title: {}", info.title);
            println!("class_name: {}", info.class_name);
            println!("pid: {}", info.pid);
            println!("x: {}", info.x);
            println!("y: {}", info.y);
            println!("width: {}", info.width);
            println!("height: {}", info.height);
        }
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
