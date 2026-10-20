use bary_core::prelude::{distance_str_v, velocity_str_v};
use bary_server::BaryServer;
use bary_sim::{World, apparent_datetime};
use clap::Parser;
use ratatui::backend::CrosstermBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, *};
use ratatui::layout::{Constraint, Margin, Rect};
use ratatui::layout::{Direction, Layout};
use ratatui::style::Style;
use ratatui::widgets::canvas::{Canvas, Circle};
use ratatui::widgets::{Block, Paragraph, Widget};
use ratatui::{Frame, Terminal};
use std::time::Duration;

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(default_value = "5000")]
    server_port: u16,
    #[arg(long, short)]
    savefile: Option<String>,
}

struct List {
    rows: Vec<String>,
    list_index: usize,
}

impl Widget for List {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical(vec![Constraint::Length(1); self.rows.len()]).split(area);

        for (i, row) in self.rows.into_iter().enumerate() {
            let mut pg = Paragraph::new(row);
            if i == self.list_index {
                pg = pg.style(Style::new().cyan());
            }
            pg.render(layout[i], buf);
        }
    }
}

fn ship_selection(frame: &mut Frame, rect: Rect, world: &World) {
    let select = List {
        rows: world
            .grids
            .values()
            .map(|grid| {
                let pos = distance_str_v(grid.particle_location.translation);
                let vel = velocity_str_v(grid.velocity.translation);
                format!("{:30} {:22} {:22}", grid.name, pos, vel)
            })
            .collect(),
        list_index: 0,
    };

    let canvas = Canvas::default()
        .block(Block::bordered())
        .x_bounds([-20.0, 20.0])
        .y_bounds([-20.0, 20.0])
        .paint(|ctx| {
            ctx.draw(&Circle {
                x: 0.0,       // Center X
                y: 0.0,       // Center Y
                radius: 10.0, // Radius of the circle
                color: ratatui::style::Color::Cyan,
            });
        });

    frame.render_widget(Block::bordered(), rect);
    frame.render_widget(canvas, rect);
    frame.render_widget(select, rect.inner(Margin::new(1, 1)));
}

fn server_loop(
    frames: u64,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut BaryServer,
) -> Result<(), Box<dyn std::error::Error>> {
    app.update();

    let mut debug_lines = Vec::new();

    debug_lines.push(format!("GUI frames: {}", frames));
    debug_lines.push(format!("Ticks: {}", app.ticks()));
    debug_lines.push(format!("World time: {}", apparent_datetime(app.ticks())));
    debug_lines.push(format!("Save file: {:?}", app.save_file()));

    debug_lines.push("CLIENTS".to_string());
    for (id, info) in app.clients() {
        debug_lines.push(format!("{}: {:?}", id, info));
    }

    let text = debug_lines.join("\n");

    terminal.draw(|frame| {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());

        let greeting = Paragraph::new(text).block(Block::bordered());
        frame.render_widget(&greeting, layout[0]);

        ship_selection(frame, layout[1], app.world());
    })?;

    std::thread::sleep(std::time::Duration::from_millis(10));
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut app = BaryServer::new(args.server_port, 1)?;

    if let Some(path) = args.savefile {
        app.load_world(path);
    }

    ratatui::run(
        |terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>| {
            let mut frames = 0;
            loop {
                server_loop(frames, terminal, &mut app)?;

                if poll(Duration::from_millis(2))? {
                    let event = read()?.as_key_event();
                    if let Some(event) = event {
                        if event.code == KeyCode::Char('q') {
                            break Ok(());
                        }
                    }
                }

                frames += 1;
            }
        },
    )
}
