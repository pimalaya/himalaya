use unicode_width::UnicodeWidthStr;

pub trait Table
where
    Self: Sized,
{
    fn head() -> Vec<String>;
    fn row(&self) -> Vec<String>;
    fn build(items: Vec<Self>) -> String {
        let mut table = vec![Self::head()];
        let mut max_col_sizes: Vec<usize> = table[0]
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
                        max_col_sizes[i] = max_col_sizes[i].max(unicode_width);
                    });
                    row
                })
                .collect::<Vec<_>>(),
        );

        table
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .enumerate()
                    .map(|(i, mut col)| {
                        let unicode_width = UnicodeWidthStr::width(col.as_str());
                        let repeat_len = max_col_sizes[i] - unicode_width + 1;
                        col.push_str(&" ".repeat(repeat_len));
                        col
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
        id: u8,
        name: String,
    }

    impl Item {
        pub fn new(id: u8, name: &str) -> Self {
            Self {
                id,
                name: name.to_owned(),
            }
        }
    }

    impl Table for Item {
        fn head() -> Vec<String> {
            vec![String::from("ID"), String::from("NAME")]
        }

        fn row(&self) -> Vec<String> {
            vec![self.id.to_string(), self.name.to_owned()]
        }
    }

    #[test]
    fn table_structure() {
        let items = vec![Item::new(1, "a"), Item::new(2, "b"), Item::new(3, "c")];

        assert_eq!(
            String::from("ID |NAME \n1  |a    \n2  |b    \n3  |c    "),
            Table::build(items)
        );
    }
}
