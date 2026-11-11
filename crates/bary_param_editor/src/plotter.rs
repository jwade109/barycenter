use egui_plot::{Legend, Line, Plot, PlotPoint, Points};
use std::collections::BTreeMap;

pub struct Plotter {
    plots: BTreeMap<String, Vec<PlotPoint>>,
}

impl Plotter {
    pub fn new() -> Self {
        Self {
            plots: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.plots.clear();
    }

    pub fn add(&mut self, key: &str, x: f64, y: f64) {
        let p = PlotPoint { x, y };
        if let Some(plot) = self.plots.get_mut(key) {
            plot.push(p);
            if plot.len() > 50 * 120 {
                plot.remove(0);
            }
        } else {
            self.plots.insert(key.to_string(), vec![p]);
        }
    }

    pub fn draw(&self, ui: &mut egui::Ui, id: &str, aspect: impl Into<Option<f32>>) {
        let legend = Legend::default();
        let plot = Plot::new(id);
        let plot = if let Some(aspect) = aspect.into() {
            plot.data_aspect(aspect)
        } else {
            plot
        };
        plot.legend(legend)
            .show_crosshair(false)
            .show(ui, |plot_ui| {
                for (name, data) in &self.plots {
                    let points = Points::new(name, data.as_slice()).radius(1.0);
                    plot_ui.points(points);
                    let series = Line::new(name, data.as_slice());
                    plot_ui.line(series);
                }
            });
    }
}
