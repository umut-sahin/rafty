use crate::*;

pub struct ScrollWidget<'debugger> {
    pub(crate) block: Block<'static>,
    pub(crate) content: &'debugger str,
    pub(crate) vertical_scroll: &'debugger mut usize,
    pub(crate) horizontal_scroll: &'debugger mut usize,
}

impl<'debugger> Widget for &mut ScrollWidget<'debugger> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        {
            let mut number_of_lines = 1;
            let mut max_line_length = 0;

            let mut current_line_length = 0;
            for char in self.content.chars() {
                if char == '\n' {
                    number_of_lines += 1;
                    if current_line_length > max_line_length {
                        max_line_length = current_line_length;
                    }
                    current_line_length = 0;
                } else if char == '\t' {
                    current_line_length += 4;
                } else if char.is_ascii_graphic() {
                    current_line_length += 1;
                }
            }
            if current_line_length > max_line_length {
                max_line_length = current_line_length;
            }

            *self.vertical_scroll = (*self.vertical_scroll).clamp(0, number_of_lines);
            *self.horizontal_scroll = (*self.horizontal_scroll).clamp(0, max_line_length);

            Paragraph::new(self.content)
                .scroll((*self.vertical_scroll as u16, *self.horizontal_scroll as u16))
                .block(self.block.clone())
                .render(area, buffer);

            let mut vertical_scroll_state =
                ScrollbarState::new(number_of_lines).position(*self.vertical_scroll);
            StatefulWidget::render(
                Scrollbar::new(ScrollbarOrientation::VerticalRight).symbols(ScrollbarSet {
                    track: "│",
                    thumb: "║",
                    begin: "╮",
                    end: "╯",
                }),
                area,
                buffer,
                &mut vertical_scroll_state,
            );

            let mut horizontal_scroll_state =
                ScrollbarState::new(max_line_length).position(*self.horizontal_scroll);
            StatefulWidget::render(
                Scrollbar::new(ScrollbarOrientation::HorizontalBottom).symbols(ScrollbarSet {
                    track: "─",
                    thumb: "═",
                    begin: "╰",
                    end: "╯",
                }),
                area,
                buffer,
                &mut horizontal_scroll_state,
            );
        }
    }
}
