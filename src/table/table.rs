use std::{cmp, fmt::Display};

pub enum ColumnAlignment {
    Left,
    #[allow(dead_code)]
    Center,
    Right,
}

pub struct TableColumn {
    alignment: ColumnAlignment,
}

impl TableColumn {
    pub fn new(alignment: ColumnAlignment) -> Self {
        Self { alignment }
    }
}

pub struct TableRow<T, const N: usize> {
    cells: [T; N],
}

impl<T, const N: usize> TableRow<T, N> {
    pub fn new(cells: [T; N]) -> Self {
        Self { cells }
    }
}

pub struct Table<T, const N: usize> {
    columns: [TableColumn; N],
    rows: Vec<TableRow<T, N>>,
}

impl<T, const N: usize> Table<T, N> {
    pub fn new(rows: Vec<TableRow<T, N>>, columns: [TableColumn; N]) -> Self {
        Self { rows, columns }
    }
}

impl<T: Display, const N: usize> Display for Table<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let column_sizes = self
            .rows
            .iter()
            .fold(vec![0; self.columns.len()], |mut res, r| {
                for col in 0..self.columns.len() {
                    res[col] = cmp::max(res[col], format!("{}", r.cells[col]).len());
                }
                return res;
            });

        for row in self.rows.iter() {
            for col in 0..self.columns.len() {
                match self.columns[col].alignment {
                    ColumnAlignment::Left => {
                        write!(f, "{:<width$} ", row.cells[col], width = column_sizes[col])?
                    }
                    ColumnAlignment::Center => {
                        write!(f, "{:^width$} ", row.cells[col], width = column_sizes[col])?
                    }
                    ColumnAlignment::Right => {
                        write!(f, "{:>width$} ", row.cells[col], width = column_sizes[col])?
                    }
                };
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}
