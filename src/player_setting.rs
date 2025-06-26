#[derive(PartialEq, Eq)]
pub enum PlayerSetting {
    Human,
    MagisterLudiAI {
        mcts_num: usize,
        mcts_parallel: usize,
        mcts_minimum: usize,
        mcts_depth: usize
    }
}