use rusty_lipgloss::{Style, writer::println};
use rusty_lipgloss as rl;
use crate::config::theme::Config;
use crate::terminal::Terminal;
use crate::components::system::Sys;
use crate::styles::ComponentStyle;

pub fn render(cfg: &Config, term: &Terminal) -> anyhow::Result<()> {
    
    let sys = Sys::new()?;
    let styles = ComponentStyle::new(cfg);

    let (_width, mut _height) = (term.width, term.height);

    // Shrink height to allow padding for bar at bottom ..(will be fixed later)
    _height = ((_height as f32) * 0.97) as u16;

    let theme = &cfg.palette;

    let labels = &["OS", "Kernel", "CPU"];
    let mut contents: Vec<String> = Vec::new();    
    
    for l in labels.iter() {
        contents.push(get_component(&sys, l));
    }
    
    let border_style = Style::new().border_foreground(&[&theme.border]);
    let header = styles.label.render("blink");
    let mut t = rl::table::Table::new()
        .headers(&[&header])
        .border(rl::Border::rounded())
        .border_style(border_style);

    for i in 0..=2 {
        t = t.row(&[
            &styles.label.clone().padding_left(1).padding_right(2).render(labels[i]),
            &styles.content.clone().padding_left(1).padding_right(2).render(&contents[i])
        ]);
    } 

     
    println(&t.render()).unwrap();

    Ok(())
}









fn get_component(sys: &Sys, key: &str) -> String {
    match key {
        "OS" => sys.os.clone(),
        "Kernel" => sys.kernel.clone(),
        "CPU" => sys.cpu.clone(),
        _ => "".into()
    }
}



