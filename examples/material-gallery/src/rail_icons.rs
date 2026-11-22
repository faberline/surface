use std::sync::OnceLock;
use surface_material::VectorIcon;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Destination {
    #[default]
    Home,
    Inbox,
    Archive,
}

impl Destination {
    pub(crate) fn icon(self) -> &'static VectorIcon {
        static HOME: OnceLock<VectorIcon> = OnceLock::new();
        static INBOX: OnceLock<VectorIcon> = OnceLock::new();
        static ARCHIVE: OnceLock<VectorIcon> = OnceLock::new();
        match self {
            Self::Home => HOME.get_or_init(|| {
                VectorIcon::new([
                    [2., 11.],
                    [12., 2.],
                    [22., 11.],
                    [20., 11.],
                    [20., 21.],
                    [14., 21.],
                    [14., 15.],
                    [10., 15.],
                    [10., 21.],
                    [4., 21.],
                    [4., 11.],
                ])
                .expect("valid home contour")
            }),
            Self::Inbox => INBOX.get_or_init(|| {
                let outer = [[2., 5.], [22., 5.], [22., 19.], [2., 19.]];
                let flap = [[5., 7.], [19., 7.], [12., 12.]];
                let pocket = [[4., 9.], [12., 14.], [20., 9.], [20., 17.], [4., 17.]];
                VectorIcon::from_contours(
                    [outer.as_slice(), flap.as_slice(), pocket.as_slice()]
                        .map(|path| path.iter().copied()),
                )
                .expect("valid inbox contours")
            }),
            Self::Archive => ARCHIVE.get_or_init(|| {
                VectorIcon::from_contours([
                    [[2., 4.], [22., 4.], [22., 8.], [2., 8.]],
                    [[4., 9.], [20., 9.], [20., 21.], [4., 21.]],
                    [[9., 12.], [15., 12.], [15., 14.], [9., 14.]],
                ])
                .expect("valid archive contours")
            }),
        }
    }
}

pub(crate) fn search() -> &'static VectorIcon {
    static SEARCH: OnceLock<VectorIcon> = OnceLock::new();
    SEARCH.get_or_init(|| {
        let ring = [
            [3., 6.],
            [6., 3.],
            [12., 3.],
            [15., 6.],
            [15., 12.],
            [12., 15.],
            [6., 15.],
            [3., 12.],
        ];
        let hole = [
            [5., 7.],
            [7., 5.],
            [11., 5.],
            [13., 7.],
            [13., 11.],
            [11., 13.],
            [7., 13.],
            [5., 11.],
        ];
        let handle = [[14., 16.], [16., 14.], [22., 20.], [20., 22.]];
        VectorIcon::from_contours(
            [ring.as_slice(), hole.as_slice(), handle.as_slice()].map(|path| path.iter().copied()),
        )
        .expect("valid search contours")
    })
}

pub(crate) fn more() -> &'static VectorIcon {
    static MORE: OnceLock<VectorIcon> = OnceLock::new();
    MORE.get_or_init(|| {
        let dots = [5., 12., 19.].map(|center| {
            [
                [center - 1., 10.],
                [center + 1., 10.],
                [center + 2., 11.],
                [center + 2., 13.],
                [center + 1., 14.],
                [center - 1., 14.],
                [center - 2., 13.],
                [center - 2., 11.],
            ]
        });
        VectorIcon::from_contours(dots).expect("valid more icon contours")
    })
}
