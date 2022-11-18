use super::main_window::MainWindow;
use super::{create_icon_button, Message, HoverListMessage, view_edit_bbs};
use iced::widget::button::Appearance;
use iced::widget::{
  horizontal_space, text_input, Column, Row, Canvas, Container, Button, Text, button, 
};
use iced::{ Alignment, Element, Length, theme, Theme };
use iced_aw::{FloatingElement, Icon};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref INPUT_ID: text_input::Id = text_input::Id::unique();
}


pub fn view_phonebook<'a>(main_window: &'a MainWindow) -> Element<'a, Message> {

    
    let list_header = Column::new()
        .push(
            Row::new()
                .push(horizontal_space(Length::Units(20)))
                .push(create_icon_button("\u{F54D}").on_press(Message::ListAction(HoverListMessage::CallBBS(0))))
                .push(horizontal_space(Length::Units(10)))
                .push(
                    text_input(
                        "Quick connect to…",
                        &main_window.addresses[0].address,
                        Message::QuickConnectChanged,
                    )
                    .id(INPUT_ID.clone())
                    .padding(8)
                    .size(18),
                )
                .push(horizontal_space(Length::Units(10)))
                .align_items(Alignment::Center),
        ).padding(8);

    let h =main_window.address_list.get_height();
    let c: Element<HoverListMessage> = Canvas::new(&main_window.address_list)
    .width(Length::Fill)
    .height(Length::Units(h)).into();

    let c = c.map(Message::ListAction);
    
    let scrollable_content = iced::widget::scrollable(iced::widget::container(c)
    .width(Length::Units(250)).center_x());

    let content = FloatingElement::new(
        scrollable_content,
        || {
            Button::new(
                Text::new(Icon::Plus.to_string())
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .font(iced_aw::ICON_FONT)
                    .size(18),
            )
            .on_press(Message::CreateNewBBS)
            .padding(5)
            .style(theme::Button::Custom(Box::new(CircleButtonStyle::new(
                theme::Button::Primary,
            ))))
            .into()
        });


    Column::new()
    .push(list_header)
    .push(Row::new()
        .push(content)
        .push(view_edit_bbs(main_window))
    )
    .padding(8)
    .spacing(8)
    .into()
}


struct CircleButtonStyle {
    theme: theme::Button,
}

impl CircleButtonStyle {
    pub fn new(theme: theme::Button) -> Self {
        Self { theme }
    }
}

impl button::StyleSheet for CircleButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        let mut appearance = style.active(&self.theme);
        appearance.border_radius = 200.0;

        appearance
    }
}