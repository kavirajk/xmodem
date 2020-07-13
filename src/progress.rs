pub enum Progress {
    Waiting,
    Started,
    Packet(u8),
}

pub type ProgressFn = fn(Progress);

pub fn noop(_: Progress) {}
