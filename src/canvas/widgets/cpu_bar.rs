
use tui::{
    Frame,
    layout::{ Direction, Rect},
};

use crate::{
    app::App,
    canvas::{
        Painter,
        drawing_utils::widget_block,
    },
};

use tui::widgets::{BarChart, BarGroup, Bar, Block};
use tui::text::Line;
use tui::symbols::bar::Set;
use tui::layout::Alignment;


impl Painter {
    /// Graphless CPU widget to be used when basic cpu is expanded
    pub fn draw_bar_cpu(
        &self, f: &mut Frame<'_>, app_state: &mut App, mut draw_loc: Rect, widget_id: u64,
    ) {
        let cpu_data = &app_state.data_store.get_data().cpu_harvest;

        if app_state.current_widget.widget_id == widget_id {
            f.render_widget(
                widget_block(true, true, self.styles.border_type)
                    .border_style(self.styles.highlighted_border_style),
                draw_loc,
            );
        }

        if draw_loc.height > 0 {

            let bars: Vec<_> = cpu_data.iter().map(|cpu|{
                let gauge = self.cpu_info(cpu);
                let start_label = gauge.0;
                let inner_label = gauge.1;
                let ratio = gauge.2 * 100.0;
                let style = gauge.3;
                Bar::default()
                    .label(Line::styled(format!("[{}", start_label), style))
                    .value(ratio as u64).value_style(style)
                    .text_value(format!("{}]", inner_label)).style(style)
            }).collect();
            let lineset = Set{full: "|", ..Default::default()};
            f.render_widget(BarChart::default()
                .block(Block::bordered().title("CPU").title_alignment(Alignment::Center))
                .bar_width(1) // TODO calc how the bar height can be
                .bar_set(lineset)
                .direction(Direction::Horizontal)
                .bar_gap(0)
                .max(100)
                .data(BarGroup::default().bars(bars.as_slice())),
                draw_loc);
        }

        if app_state.should_get_widget_bounds() {
            // Update draw loc in widget map
            if let Some(widget) = app_state.widget_map.get_mut(&widget_id) {
                widget.top_left_corner = Some((draw_loc.x, draw_loc.y));
                widget.bottom_right_corner =
                    Some((draw_loc.x + draw_loc.width, draw_loc.y + draw_loc.height));
            }
        }
    }
}
