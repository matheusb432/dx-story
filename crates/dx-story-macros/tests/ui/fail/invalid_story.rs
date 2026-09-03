use facade::story;

#[story(name = "Invalid")]
async fn InvalidStory(value: usize) {
    let _ = value;
}

fn main() {}
