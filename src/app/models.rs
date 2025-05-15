use eframe::egui::{Color32, text::LayoutJob, FontId};

#[derive(Clone)]
pub struct LogEntry {
    pub text: String,
    pub color: Color32,
    pub timestamp: String,
}

impl LogEntry {
    pub fn to_layout_job(&self) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.append(
            &format!("[{}] ", self.timestamp),
            0.0,
            Self::timestamp_format(),
        );
        job.append(
            &self.text,
            0.0,
            Self::text_format(self.color),
        );
        job
    }

    fn timestamp_format() -> eframe::egui::TextFormat {
        eframe::egui::TextFormat::simple(
            FontId::monospace(13.0),
            Color32::GRAY,
        )
    }

    fn text_format(color: Color32) -> eframe::egui::TextFormat {
        eframe::egui::TextFormat::simple(
            FontId::monospace(13.0),
            color,
        )
    }
}