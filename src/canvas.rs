use crate::{Component, ComponentId, Draw, DrawMetadata};
use itertools::Position;
use ratatui::{
    buffer::Buffer,
    layout::{Offset, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{StatefulWidget, Widget},
};
use std::{collections::HashMap, mem, time::Instant};

/// A wrapper around a [Buffer] that manages draw state for a single frame of
/// drawing.
///
/// TODO move this into a module
#[derive(derive_more::Debug)]
#[must_use = "Call .into_component_map() to get rendered components"]
pub struct Canvas<'buf> {
    /// Main frame buffer
    buffer: &'buf mut Buffer,
    /// Position of the terminal cursor. `None` if the cursor should be hidden.
    /// It's shown only when the user can type
    cursor_position: Option<Position>,
    /// Throughout a draw, we track which components are drawn and where. At
    /// the end of the draw, this is returned to the caller so it can be used
    /// during the subsequent update phase.
    components: ComponentMap,
    /// Track and draw render times on each component?
    draw_render_times: bool,
}

impl<'buf> Canvas<'buf> {
    /// Wrap a frame for a single walk down the draw tree
    pub fn new(buffer: &'buf mut Buffer) -> Self {
        Self {
            buffer,
            cursor_position: None,
            components: ComponentMap::default(),
            draw_render_times: false,
        }
    }

    /// Enable/disable render time tracking
    ///
    /// When enabled, the time it takes to draw each component will be rendered
    /// in the corner of that component.
    pub fn with_render_times(mut self, enable: bool) -> Self {
        self.draw_render_times = enable;
        self
    }

    /// Create a new canvas and draw an entire component tree to it
    ///
    /// Return the drawn canvas; you probably want to call
    /// [Self::into_component_map] on it.
    pub fn draw_all<T, X, Props>(
        buffer: &'buf mut Buffer,
        root: &T,
        props: Props,
    ) -> Self
    where
        T: Component<X> + Draw<Props>,
    {
        Self::draw_all_area(buffer, root, props, *buffer.area(), true)
    }

    /// [Self::draw_all], but the caller determines the area and focus of the
    /// root component
    ///
    /// Called directly only for tests, where those need to be configured.
    /// Return the drawn canvas; you probably want to call
    /// [Self::into_component_map] on it.
    pub fn draw_all_area<T, X, Props>(
        buffer: &'buf mut Buffer,
        root: &T,
        props: Props,
        area: Rect,
        has_focus: bool,
    ) -> Self
    where
        T: Component<X> + Draw<Props>,
    {
        let mut canvas = Self::new(buffer);
        canvas.draw(root, props, area, has_focus);
        canvas
    }

    /// Draw a component to the screen
    ///
    /// ## Params
    ///
    /// - `component`: Component to draw
    /// - `props`: Arbitrary data to pass to the component's `draw()` method
    /// - `area`: Area of the screen to draw the component to
    /// - `has_focus`: Should this component receive future keyboard events?
    pub fn draw<T, X, Props>(
        &mut self,
        component: &T,
        props: Props,
        area: Rect,
        has_focus: bool,
    ) where
        T: Component<X> + Draw<Props> + ?Sized,
    {
        let metadata = DrawMetadata { area, has_focus };

        // Mark this component as visible so it can receive events
        self.components.0.insert(component.id(), metadata);

        let start = Instant::now();
        component.draw(self, props, metadata);
        let elapsed = start.elapsed();
        // In debug mode, show the draw time for each component
        if self.draw_render_times {
            // Use Span instead of Line so it doesn't cover the whole line
            let text = Span::styled(
                format!("{}μs", elapsed.as_micros()),
                Style::default().fg(Color::Black).bg(Color::Green),
            );
            let width = text.width() as u16;
            // Bottom-right corner of the component
            let area = Rect {
                x: area.right() - width,
                y: area.bottom() - 1,
                width,
                height: 1,
            };
            Widget::render(text, area, self.buffer);
        }
    }

    /// Get the full screen area
    pub fn area(&self) -> Rect {
        self.buffer.area
    }

    /// Get a mutable reference to the internal screen buffer
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.buffer
    }

    /// Get the desired position of the terminal cursor, or `None` if it should
    /// be hidden
    pub fn cursor_position(&self) -> Option<Position> {
        self.cursor_position
    }

    /// Show the cursor at the given position
    pub fn set_cursor_position(&mut self, position: Position) {
        self.cursor_position = Some(position);
    }

    /// Render a [Widget] to the active buffer
    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer);
    }

    /// Render a [StatefulWidget] to the active buffer
    pub fn render_stateful_widget<W>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) where
        W: StatefulWidget,
    {
        widget.render(area, self.buffer, state);
    }

    /// Copy one sub-area of one canvas into a sub-area of another
    ///
    /// Use this to complete the rendering of a virtual canvas. The source
    /// canvas's contents within the `from` area will be copied to the `to`
    /// area. The canvas's other internal state, including visible components,
    /// will be merged as well.
    ///
    /// ## Panics
    ///
    /// Panic if `from` and `to` are not the same size.
    pub fn merge(&mut self, other: Canvas, from: Rect, to: Rect) {
        // Safety first!
        debug_assert_eq!(
            from.as_size(),
            to.as_size(),
            "Source and target areas are not the same size"
        );

        // Copy the other buffer's contents to our own. We know the two areas
        // are the same size, so the positions() iters will be the same length
        //
        // It's possible this would be faster if we went by row instead of by
        // cell. I don't think there's any way to do mem::take on entire rows
        // at a time, so it would involve cloning. I haven't tested it.
        for (from, to) in from.positions().zip(to.positions()) {
            self.buffer[to] = mem::take(&mut other.buffer[from]);
        }

        // Merge other state
        self.components.0.extend(other.components.0);
        // Other canvas gets priority, so takes its cursor first
        self.cursor_position = other
            .cursor_position
            .map(|pos| {
                // Shift from source to absolute, then to target
                let from: Offset = from.as_position().into();
                let to: Offset = to.as_position().into();
                pos - from + to
            })
            .or(self.cursor_position);
    }

    /// Get the map of components that were visible in this canvas's draw
    pub fn into_component_map(self) -> ComponentMap {
        self.components
    }
}

/// All components that were drawn during the most recent draw phase
///
/// A new map is built for each [Canvas::draw_all] call, which means a new map
/// every draw frame.
///
/// The purpose of this is to allow each component to return an exhaustive list
/// of its children during event handling, then we can automatically filter that
/// list down to just the ones that are visible. This prevents the need to
/// duplicate visibility logic in both the draw and the children getters.
/// For each drawn component, this stores metadata related to its last
/// draw.
#[derive(Debug, Default)]
#[must_use = "Store component map to update visibility state"]
pub struct ComponentMap(HashMap<ComponentId, DrawMetadata>);

impl ComponentMap {
    /// Get the area that the component was drawn to. Return `None` iff the
    /// component is not visible.
    pub fn area<T: Component + ?Sized>(&self, component: &T) -> Option<Rect> {
        self.0.get(&component.id()).map(|metadata| metadata.area())
    }

    /// Was this component in focus during the previous draw phase?
    pub fn has_focus<T: Component + ?Sized>(&self, component: &T) -> bool {
        let metadata = self.0.get(&component.id());
        metadata.is_some_and(|metadata| metadata.has_focus())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        UpdateContext,
        event::{Event, EventMatch},
        test_util::{TestHarness, harness},
    };
    use ratatui::{layout::Size, text::Line};
    use rstest::rstest;
    use std::collections::HashSet;

    #[derive(Debug, Default)]
    struct Comp {
        id: ComponentId,
        /// How many events have we consumed?
        count: u32,
    }

    impl Comp {
        fn reset(&mut self) {
            self.count = 0;
        }
    }

    impl Component<()> for Comp {
        fn id(&self) -> ComponentId {
            self.id
        }

        fn update(&mut self, _: &mut UpdateContext, _: Event) -> EventMatch {
            self.count += 1;
            None.into()
        }
    }

    impl Draw for Comp {
        fn draw(&self, canvas: &mut Canvas, (): (), metadata: DrawMetadata) {
            canvas.render_widget("hello!", metadata.area());
        }
    }

    /// Test merging two canvases together
    /// - Only content from the source area is copied
    /// - Content is copied to the target area of the main canvas
    /// - Component maps are merged
    #[rstest]
    #[case::full_nonoverlapping(
        Rect { x: 0, y: 0, width: 6, height: 1 },
        Rect { x: 0, y: 1, width: 6, height: 1 },
        ["hello!", "hello!"],
    )]
    #[case::partial_overwrite(
        Rect { x: 0, y: 0, width: 3, height: 1 },
        Rect { x: 3, y: 0, width: 3, height: 1 },
        ["helhel", "      "],
    )]
    fn test_merge<'a>(
        _harness: TestHarness,
        #[case] from: Rect,
        #[case] to: Rect,
        #[case] expected_content: impl IntoIterator<Item = impl Into<Line<'a>>>,
    ) {
        let component1 = Comp::default();
        let mut buffer1 = Buffer::empty(Size::new(6, 2).into());
        let mut canvas1 = Canvas::new(&mut buffer1);
        canvas1.draw(&component1, (), canvas1.area(), true);

        let component2 = Comp::default();
        let mut buffer2 = Buffer::empty(Size::new(6, 1).into());
        let mut canvas2 = Canvas::new(&mut buffer2);
        canvas2.draw(&component2, (), canvas2.area(), true);

        // DO IT
        canvas1.merge(canvas2, from, to);

        // Visible component maps were merged
        assert_eq!(
            canvas1.components.0.into_keys().collect::<HashSet<_>>(),
            [component1.id(), component2.id()]
                .into_iter()
                .collect::<HashSet<_>>()
        );
        // Buffer equals expectation
        let expected = Buffer::with_lines(expected_content);
        assert_eq!(buffer1, expected);
    }

    /// Canvas::merge picks up the right cursor position, giving priority to the
    /// incoming canvas and shifting its position appropriately
    #[rstest]
    #[case::none_none(None, None, None)]
    #[case::none_some(None, Some((1, 4).into()), Some((1, 4).into()))]
    #[case::some_none(Some((3, 1).into()), None, Some((4, 4).into()))]
    #[case::some_some(
        // Take the first cursor, shifted by the source origin
        Some((3, 1).into()), Some((1, 4).into()), Some((4, 4).into()),
    )]
    fn test_merge_cursor(
        _harness: TestHarness,
        #[case] from_cursor: Option<Position>,
        #[case] to_cursor: Option<Position>,
        #[case] expected: Option<Position>,
    ) {
        let mut buffer1 = Buffer::empty(Size::new(5, 5).into());
        let mut canvas1 = Canvas::new(&mut buffer1);
        if let Some(cursor) = to_cursor {
            canvas1.set_cursor_position(cursor);
        }

        let mut buffer2 = Buffer::empty(Size::new(5, 5).into());
        let mut canvas2 = Canvas::new(&mut buffer2);
        if let Some(cursor) = from_cursor {
            canvas2.set_cursor_position(cursor);
        }

        let width = 1;
        let height = 1;
        canvas1.merge(
            canvas2,
            // Map the cursor to the bottom-right corner
            Rect {
                x: 3,
                y: 1,
                width,
                height,
            },
            Rect {
                x: 4,
                y: 4,
                width,
                height,
            },
        );

        assert_eq!(canvas1.cursor_position(), expected);
    }

    /// Merging panics if the source and target areas are different sizes
    #[rstest]
    #[should_panic(expected = "Source and target areas are not the same size")]
    fn test_merge_panic(_harness: TestHarness) {
        let mut buffer1 = Buffer::empty(Size::new(3, 3).into());
        let mut canvas1 = Canvas::new(&mut buffer1);
        let mut buffer2 = Buffer::empty(Size::new(3, 3).into());
        let canvas2 = Canvas::new(&mut buffer2);
        canvas1.merge(canvas2, Size::new(2, 2).into(), Size::new(2, 1).into());
    }
}
