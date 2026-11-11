use egui_plot::{Legend, Line, Plot, PlotPoint, PlotPoints, Points};
use std::collections::BTreeMap;

pub struct Plotter {
    id: String,
    aspect: Option<f32>,
    plots: BTreeMap<String, Vec<PlotPoint>>,
}

impl Plotter {
    pub fn new(id: impl Into<String>, aspect: impl Into<Option<f32>>) -> Self {
        Self {
            id: id.into(),
            aspect: aspect.into(),
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
            if plot.len() > 50 * 30 {
                plot.remove(0);
            }
        } else {
            self.plots.insert(key.to_string(), vec![p]);
        }
    }

    pub fn draw(&self, ui: &mut egui::Ui) {
        let legend = Legend::default();
        let plot = Plot::new(&self.id);
        let plot = if let Some(aspect) = self.aspect.into() {
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
                    if let Some(last) = data.last() {
                        let series = PlotPoints::Owned(vec![*last]);
                        let last = Points::new(name, series).radius(4.0);
                        plot_ui.points(last);
                    }
                }
            });
    }
}
