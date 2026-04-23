use super::state::{AppState, Status};
use crate::interview::history::Speaker;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::time::Duration;

pub fn render_frame(f: &mut Frame, state: &AppState, elapsed: Duration) {
    let area = f.size();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

    render_top_bar(f, outer[0], state, elapsed);
    render_main(f, outer[1], state);
    render_bottom_bar(f, outer[2]);
}

fn render_top_bar(f: &mut Frame, area: Rect, state: &AppState, elapsed: Duration) {
    let title = format!(" {} ", state.title());
    let elapsed_s = format!(" {} ", state.format_elapsed(elapsed));
    let pad = area
        .width
        .saturating_sub(title.len() as u16 + elapsed_s.len() as u16) as usize;
    let line = Line::from(vec![
        Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(pad)),
        Span::styled(elapsed_s, Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_main(f: &mut Frame, area: Rect, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    render_transcript(f, cols[0], state);
    render_sidebar(f, cols[1], state);
}

fn render_transcript(f: &mut Frame, area: Rect, state: &AppState) {
    let mut lines: Vec<Line> = Vec::new();
    for turn in state.history() {
        let (glyph, color) = match turn.speaker {
            Speaker::Conch => ("✣ Conch", Color::Magenta),
            Speaker::User => ("● You", Color::Green),
        };
        lines.push(Line::from(Span::styled(
            glyph,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        for wrapped in textwrap(&turn.text, area.width.saturating_sub(4) as usize) {
            lines.push(Line::from(Span::raw(format!("  {}", wrapped))));
        }
        lines.push(Line::from(""));
    }
    if !state.current_user_draft().is_empty() {
        lines.push(Line::from(Span::styled(
            "● You",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::DIM),
        )));
        for wrapped in textwrap(
            state.current_user_draft(),
            area.width.saturating_sub(4) as usize,
        ) {
            lines.push(Line::from(Span::styled(
                format!("  {}", wrapped),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
    }
    if !state.assistant_stream().is_empty() {
        lines.push(Line::from(Span::styled(
            "✣ Conch",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        for wrapped in textwrap(
            state.assistant_stream(),
            area.width.saturating_sub(4) as usize,
        ) {
            lines.push(Line::from(Span::raw(format!("  {}", wrapped))));
        }
        lines.push(Line::from(Span::styled(
            "  ▋",
            Style::default().fg(Color::Magenta),
        )));
    }
    let block = Block::default().borders(Borders::ALL).title(" Transcript ");
    let take = lines
        .len()
        .saturating_sub(area.height.saturating_sub(2) as usize);
    let view: Vec<Line> = lines.into_iter().skip(take).collect();
    f.render_widget(Paragraph::new(view).block(block), area);
}

fn render_sidebar(f: &mut Frame, area: Rect, state: &AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    let brief_block = Block::default().borders(Borders::ALL).title(" Brief ");
    f.render_widget(
        Paragraph::new(state.brief())
            .block(brief_block)
            .wrap(Wrap { trim: false }),
        rows[0],
    );

    let wave = render_waveform(state.waveform(), rows[1].width.saturating_sub(2) as usize);
    f.render_widget(
        Paragraph::new(wave).block(Block::default().borders(Borders::ALL)),
        rows[1],
    );

    let status_line = status_label(state.status(), state.banner());
    f.render_widget(
        Paragraph::new(status_line).block(Block::default().borders(Borders::ALL)),
        rows[2],
    );
}

fn render_bottom_bar(f: &mut Frame, area: Rect) {
    f.render_widget(
        Paragraph::new(
            " [Hold Space] talk  [Tap Space] toggle mic  [Esc] interrupt  [Ctrl+C] quit ",
        ),
        area,
    );
}

fn render_waveform(samples: &[f32], cols: usize) -> Line<'static> {
    const BARS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    if samples.is_empty() || cols == 0 {
        return Line::from(Span::raw(""));
    }
    let step = (samples.len() as f32 / cols as f32).max(1.0);
    let mut s = String::with_capacity(cols);
    let mut i = 0.0;
    while (i as usize) < samples.len() && s.chars().count() < cols {
        let v = samples[i as usize].clamp(0.0, 1.0);
        let idx = (v * 7.0).round() as usize;
        s.push_str(BARS[idx.min(7)]);
        i += step;
    }
    Line::from(Span::styled(s, Style::default().fg(Color::Cyan)))
}

fn status_label(status: Status, banner: Option<&str>) -> Line<'static> {
    if let Some(b) = banner {
        return Line::from(Span::styled(
            b.to_string(),
            Style::default().fg(Color::Yellow),
        ));
    }
    let (glyph, text, color) = match status {
        Status::Idle => ("✶", "Idle — hold Space to talk", Color::Gray),
        Status::Listening => ("✻", "Listening\u{2026}", Color::Green),
        Status::Thinking => ("✶", "Thinking\u{2026}", Color::Yellow),
        Status::Filling => ("✶", "Thinking\u{2026}", Color::Yellow),
        Status::Speaking => ("✣", "Speaking\u{2026}", Color::Magenta),
        Status::Filler => ("✶", "\u{2026}", Color::Yellow),
        Status::Closing => ("✣", "Wrapping up\u{2026}", Color::Magenta),
    };
    Line::from(vec![
        Span::styled(format!("{} ", glyph), Style::default().fg(color)),
        Span::styled(text.to_string(), Style::default().fg(color)),
    ])
}

fn textwrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.chars().count() + word.chars().count() + 1 > width && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
