use facade::preview;

#[preview(name = "Invalid")]
async fn InvalidPreview(value: usize) {
    let _ = value;
}

fn main() {}
