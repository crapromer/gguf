﻿use crate::{
    gguf_file::{pad, GGufFile},
    loose_shards::LooseShards,
};
use ggus::{GGufFileHeader, GGufWriter};
use indexmap::{IndexMap, IndexSet};
use std::{fs::File, iter::zip, path::PathBuf, thread};

#[derive(Args, Default)]
pub struct MergeArgs {
    /// One of the shards to merge.
    file: PathBuf,
    /// Output directory for merged file
    #[clap(long, short)]
    output_dir: Option<PathBuf>,
}

impl MergeArgs {
    pub fn merge(self) {
        // 检查输入文件是否分片
        let shards = LooseShards::from(&*self.file);
        if shards.count() < 2 {
            println!("Model does not need to merge.");
            return;
        }
        // 打开所有分片文件
        let mut files = Vec::new();
        for path in &shards {
            match File::open(&path) {
                Ok(file) => files.push(unsafe { memmap2::Mmap::map(&file).unwrap() }),
                Err(e) => {
                    eprintln!("Failed to open");
                    eprintln!("  file: {}", path.display());
                    eprintln!("  cause: {e}");
                    return;
                }
            }
        }
        // 解析所有分片文件
        let files = thread::scope(|s| {
            files
                .iter()
                .map(|data| s.spawn(|| GGufFile::new(data).unwrap()))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|t| t.join().unwrap())
                .collect::<Vec<_>>()
        });
        // 扫描并合并元数据键值对和张量信息
        let kvs = files
            .iter()
            .flat_map(|file| file.meta_kvs.kvs())
            .filter(|kv| {
                let key = kv.key();
                !key.starts_with("split.") && key != "general.alignment"
            })
            .collect::<IndexSet<_>>();
        let tensors = files
            .iter()
            .enumerate()
            .flat_map(|(i, file)| file.tensors.iter().map(move |t| (t, i)))
            .collect::<IndexMap<_, _>>();
        // 设置输出目录
        let mut file_format = shards;
        if let Some(dir) = self.output_dir {
            if let Err(e) = std::fs::create_dir_all(&dir) {
                panic!("Failed to create output directory: {e}");
            }
            file_format.dir = dir;
        }
        // 计算对齐
        let align = files
            .iter()
            .map(|file| file.meta_kvs.alignment())
            .max()
            .unwrap();
        // 写入合并后的文件
        let out = File::create(file_format.single_file()).unwrap();
        let header = GGufFileHeader::new(3, tensors.len() as _, (kvs.len() + 1) as _);
        let mut writer = GGufWriter::new(out, header).unwrap();

        writer.write_alignment(align).unwrap();
        for kv in kvs {
            writer
                .write_meta_kv(kv.key(), kv.ty(), kv.value_bytes())
                .unwrap();
        }

        let mut cursor = 0;
        let mut paddings = Vec::with_capacity(tensors.len() + 1);
        paddings.push(0);

        for t in tensors.keys() {
            writer
                .write_tensor_info(t.name(), t.shape(), t.ggml_type(), cursor)
                .unwrap();

            cursor += t.nbytes();
            let padding = pad(cursor, align);

            cursor += padding;
            paddings.push(padding);
        }

        paddings.pop();
        paddings[0] = pad(writer.written_bytes(), align);

        for ((t, i), padding) in zip(tensors, paddings) {
            for _ in 0..padding {
                writer.write(0u8).unwrap();
            }
            writer
                .write_bytes(&files[i].data[t.offset()..][..t.nbytes()])
                .unwrap();
        }
    }
}
