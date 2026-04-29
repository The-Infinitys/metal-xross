use crate::effector::MetalXross;
use truce::prelude::*;
impl PluginLogic for MetalXross {
    /// nih_plugの initialize に相当します
    fn reset(&mut self, sample_rate: f64, max_block_size: usize) {
        self.reset(sample_rate, max_block_size);
    }

    /// 音声処理のメインループ
    fn process(
        &mut self,
        buffer: &mut AudioBuffer,
        events: &EventList,
        context: &mut ProcessContext,
    ) -> ProcessStatus {
        events
            .iter()
            .map(|event| &event.body)
            .for_each(|event| match event {
                EventBody::ParamChange { id, value } => {
                    self.params().set_normalized(*id, *value);
                }

                // MIDIノートなどは必要に応じて
                EventBody::NoteOn { .. } => { /* ... */ }

                // それ以外のイベントは無視するか適切に処理
                _ => {}
            });
        // nih_plugの process(buffer, aux, context) からの移植
        // buffer.as_slice() などでデータにアクセスして処理します
        self.process(buffer, events, context)
    }
    fn custom_editor(&self) -> Option<Box<dyn Editor>> {
        Some(self.ui())
    }
}
