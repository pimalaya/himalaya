use unicode_width::UnicodeWidthStr;

pub trait Table
where
    Self: Sized,
{
    fn head() -> Vec<String>;
    fn row(&self) -> Vec<String>;
    fn shrink_col_index() -> usize;
    fn max_width() -> usize;
    fn build(items: Vec<Self>) -> String {
        let mut table = vec![Self::head()];
        let mut max_col_widths: Vec<usize> = table[0]
            .iter()
            .map(|col| UnicodeWidthStr::width(col.as_str()))
            .collect();
        table.extend(
            items
                .iter()
                .map(|item| {
                    let row = item.row();
                    row.iter().enumerate().for_each(|(i, col)| {
                        let unicode_width = UnicodeWidthStr::width(col.as_str());
                        max_col_widths[i] = max_col_widths[i].max(unicode_width);
                    });
                    row
                })
                .collect::<Vec<_>>(),
        );

        let table_width = max_col_widths.iter().sum::<usize>() + max_col_widths.len() * 2 - 1;

        table
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .enumerate()
                    .map(|(i, mut col)| {
                        if i == Self::shrink_col_index() && table_width > Self::max_width() {
                            let shrink_width = table_width - Self::max_width();
                            let max_col_width = max_col_widths[i] - shrink_width;
                            if max_col_width < UnicodeWidthStr::width(col.as_str()) {
                                let mut next_col = String::new();
                                let mut cur = 0;

                                for c in col.chars() {
                                    let unicode_width =
                                        UnicodeWidthStr::width(c.to_string().as_str());
                                    if cur + unicode_width >= max_col_width {
                                        break;
                                    }

                                    cur += unicode_width;
                                    next_col.push(c);
                                }

                                next_col.push_str("… ");
                                next_col.push_str(&" ".repeat(max_col_width - cur - 1));
                                next_col
                            } else {
                                let unicode_width = UnicodeWidthStr::width(col.as_str());
                                let repeat_len = max_col_width - unicode_width + 1;
                                col.push_str(&" ".repeat(repeat_len));
                                col
                            }
                        } else {
                            let unicode_width = UnicodeWidthStr::width(col.as_str());
                            let repeat_len = max_col_widths[i] - unicode_width + 1;
                            col.push_str(&" ".repeat(repeat_len));
                            col
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::Table;

    struct Item {
        id: u16,
        name: String,
        desc: String,
    }

    impl Item {
        pub fn new(id: u16, name: &str, desc: &str) -> Self {
            Self {
                id,
                name: name.to_owned(),
                desc: desc.to_owned(),
            }
        }
    }

    impl Table for Item {
        fn head() -> Vec<String> {
            vec![
                String::from("ID"),
                String::from("NAME"),
                String::from("DESC"),
            ]
        }

        fn row(&self) -> Vec<String> {
            vec![
                self.id.to_string(),
                self.name.to_owned(),
                self.desc.to_owned(),
            ]
        }

        fn shrink_col_index() -> usize {
            1
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
        let table = "
ID |NAME |DESC 
1  |a    |aa   
2  |b    |bb   
3  |c    |cc   
";

        assert_eq!(String::from(table.trim_matches('\n')), Table::build(items));
    }

    #[test]
    fn row_bigger_than_head() {
        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "c", "cc"),
        ];
        let table = "
ID   |NAME  |DESC  
1    |a     |aa    
2222 |bbbbb |bbbbb 
3    |c     |cc    
";

        assert_eq!(String::from(table.trim_matches('\n')), Table::build(items));

        let items = vec![
            Item::new(1, "a", "aa"),
            Item::new(2222, "bbbbb", "bbbbb"),
            Item::new(3, "cccccc", "cc"),
        ];
        let table = "
ID   |NAME   |DESC  
1    |a      |aa    
2222 |bbbbb  |bbbbb 
3    |cccccc |cc    
";

        assert_eq!(String::from(table.trim_matches('\n')), Table::build(items));
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

        let table = "
ID |NAME      |DESC 
1  |short     |desc 
2  |loooooong |desc 
3  |shriiiii… |desc 
4  |shriiiii… |desc 
5  |😍😍😍😍  |desc 
6  |😍😍😍😍… |desc 
7  |!😍😍😍…  |desc 
";

        assert_eq!(String::from(table.trim_matches('\n')), Table::build(items));
    }
}
