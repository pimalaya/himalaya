use std::fmt;
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub struct Style(u8, u8, u8);

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Style(color, bright, shade) = self;
        let mut style = String::from("\x1b[");

        style.push_str(&color.to_string());

        if *bright > 0 {
            style.push_str(";");
            style.push_str(&bright.to_string());
        };

        if *shade > 0 {
            style.push_str(";");
            style.push_str(&shade.to_string());
        };

        style.push_str("m");

        write!(f, "{}", style)
    }
}

#[derive(Debug)]
pub struct Cell {
    styles: Vec<Style>,
    value: String,
    shrinkable: bool,
}

impl Cell {
    pub fn new<T: AsRef<str>>(value: T) -> Self {
        Self {
            styles: Vec::new(),
            value: String::from(value.as_ref()),
            shrinkable: false,
        }
    }

    pub fn unicode_width(&self) -> usize {
        UnicodeWidthStr::width(self.value.as_str())
    }

    pub fn shrinkable(mut self) -> Self {
        self.shrinkable = true;
        self
    }

    pub fn is_shrinkable(&self) -> bool {
        self.shrinkable
    }

    pub fn bold(mut self) -> Self {
        self.styles.push(Style(1, 0, 0));
        self
    }

    pub fn underline(mut self) -> Self {
        self.styles.push(Style(4, 0, 0));
        self
    }

    pub fn red(mut self) -> Self {
        self.styles.push(Style(31, 0, 0));
        self
    }

    pub fn green(mut self) -> Self {
        self.styles.push(Style(32, 0, 0));
        self
    }

    pub fn yellow(mut self) -> Self {
        self.styles.push(Style(33, 0, 0));
        self
    }

    pub fn blue(mut self) -> Self {
        self.styles.push(Style(34, 0, 0));
        self
    }

    pub fn white(mut self) -> Self {
        self.styles.push(Style(37, 0, 0));
        self
    }

    pub fn ext(mut self, shade: u8) -> Self {
        self.styles.push(Style(38, 5, shade));
        self
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for style in &self.styles {
            write!(f, "{}", style)?;
        }

        write!(f, "{}", self.value)?;

        if !self.styles.is_empty() {
            write!(f, "{}", Style(0, 0, 0))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Row(pub Vec<Cell>);

impl Row {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn cell(mut self, cell: Cell) -> Self {
        self.0.push(cell);
        self
    }
}

pub trait Table
where
    Self: Sized,
{
    fn head() -> Row;
    fn row(&self) -> Row;

    fn max_width() -> usize {
        terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or_default()
    }

    fn build(items: &[Self]) -> Vec<Vec<String>> {
        let mut table = vec![Self::head()];
        let mut cell_widths: Vec<usize> =
            table[0].0.iter().map(|cell| cell.unicode_width()).collect();
        table.extend(
            items
                .iter()
                .map(|item| {
                    let row = item.row();
                    row.0.iter().enumerate().for_each(|(i, cell)| {
                        cell_widths[i] = cell_widths[i].max(cell.unicode_width());
                    });
                    row
                })
                .collect::<Vec<_>>(),
        );

        let spaces_plus_separators_len = cell_widths.len() * 2 - 1;
        let table_width = cell_widths.iter().sum::<usize>() + spaces_plus_separators_len;

        table
            .iter_mut()
            .map(|row| {
                row.0
                    .iter_mut()
                    .enumerate()
                    .map(|(i, cell)| {
                        let table_is_overflowing = table_width > Self::max_width();

                        if table_is_overflowing && cell.is_shrinkable() {
                            let shrink_width = table_width - Self::max_width();
                            let cell_width = cell_widths[i] - shrink_width;
                            let cell_is_overflowing = cell.unicode_width() > cell_width;

                            if cell_is_overflowing {
                                let mut value = String::new();
                                let mut chars_width = 0;

                                for c in cell.value.chars() {
                                    let char_width = UnicodeWidthStr::width(c.to_string().as_str());
                                    if chars_width + char_width >= cell_width {
                                        break;
                                    }

                                    chars_width += char_width;
                                    value.push(c);
                                }

                                value.push_str("… ");
                                let repeat_count = cell_width - chars_width - 1;
                                value.push_str(&" ".repeat(repeat_count));
                                cell.value = value;
                                cell.to_string()
                            } else {
                                let repeat_len = cell_width - cell.unicode_width() + 1;
                                cell.value.push_str(&" ".repeat(repeat_len));
                                cell.to_string()
                            }
                        } else {
                            let repeat_count = cell_widths[i] - cell.unicode_width() + 1;
                            cell.value.push_str(&" ".repeat(repeat_count));
                            cell.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    fn render(items: &[Self]) -> String {
        Self::build(items)
            .iter()
            .map(|row| row.join(&Cell::new("|").ext(8).to_string()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        id: u16,
        name: String,
        desc: String,
    }

    impl<'a> Item {
        pub fn new(id: u16, name: &'a str, desc: &'a str) -> Self {
            Self {
                id,
                name: String::from(name),
                desc: String::from(desc),
            }
        }
    }

    impl Table for Item {
        fn head() -> Row {
            Row::new()
                .cell(Cell::new("ID"))
                .cell(Cell::new("NAME").shrinkable())
                .cell(Cell::new("DESC"))
        }

        fn row(&self) -> Row {
            Row::new()
                .cell(Cell::new(self.id.to_string()))
                .cell(Cell::new(self.name.as_str()).shrinkable())
                .cell(Cell::new(self.desc.as_str()))
        }

        fn max_width() -> usize {
            20
        }
    }

    #[test]
    fn row_smaller_than_head() {
        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2, "b", "bb"),
            Item::new(3, "c", "cc"),
        ];

        let table = vec![
            vec!["ID ", "NAME ", "DESC "],
            vec!["1  ", "a    ", "aa   "],
            vec!["2  ", "b    ", "bb   "],
            vec!["3  ", "c    ", "cc   "],
        ];

        assert_eq!(table, Table::build(&items));
    }

    #[test]
    fn row_bigger_than_head() {
        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "c", "cc"),
        ];

        let table = vec![
            vec!["ID   ", "NAME  ", "DESC  "],
            vec!["1    ", "a     ", "aa    "],
            vec!["2222 ", "bbbbb ", "bbbbb "],
            vec!["3    ", "c     ", "cc    "],
        ];

        assert_eq!(table, Table::build(&items));

        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "cccccc", "cc"),
        ];

        let table = vec![
            vec!["ID   ", "NAME   ", "DESC  "],
            vec!["1    ", "a      ", "aa    "],
            vec!["2222 ", "bbbbb  ", "bbbbb "],
            vec!["3    ", "cccccc ", "cc    "],
        ];

        assert_eq!(table, Table::build(&items));
    }

    #[test]
    fn shrink() {
        let items = vec![
            Item::new(1, "short", "desc"),
            Item::new(2, "loooooong", "desc"),
            Item::new(3, "shriiiiink", "desc"),
            Item::new(4, "shriiiiiiiiiink", "desc"),
            Item::new(5, "😍😍😍😍", "desc"),
            Item::new(6, "😍😍😍😍😍", "desc"),
            Item::new(7, "!😍😍😍😍😍", "desc"),
        ];

        let table = vec![
            vec!["ID ", "NAME      ", "DESC "],
            vec!["1  ", "short     ", "desc "],
            vec!["2  ", "loooooong ", "desc "],
            vec!["3  ", "shriiiii… ", "desc "],
            vec!["4  ", "shriiiii… ", "desc "],
            vec!["5  ", "😍😍😍😍  ", "desc "],
            vec!["6  ", "😍😍😍😍… ", "desc "],
            vec!["7  ", "!😍😍😍…  ", "desc "],
        ];

        assert_eq!(table, Table::build(&items));
    }
}
