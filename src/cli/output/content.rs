use super::{
    CompiledQuery, ContentEvent, ContentFormat, ContentReceiver, ContentRecord, ContentRecordKind,
    ContentSummary, ControlFlow, Decoration, FormatOptions, Options, Output, Path, PathBuf, Row,
    ScanStats, Stats, Visitor, Write, content_inputs, fs, io, write_path,
};

pub(super) struct PendingContent {
    display: PathBuf,
    absolute: PathBuf,
    metadata: Option<fs::Metadata>,
    eligibility: Option<String>,
    receiver: ContentReceiver,
}

struct FileOutput {
    display: PathBuf,
    absolute: PathBuf,
    metadata: Option<fs::Metadata>,
    eligibility: Option<String>,
}

impl<W: Write> Output<W> {
    pub(crate) fn set_content_plan(&mut self, plan: &CompiledQuery) {
        if self.options.content_options.explain_matches && self.options.search_paths.is_empty() {
            self.content_plan = Some(plan.clone());
        }
    }

    pub(crate) fn scan_stdin(&mut self) -> io::Result<()> {
        let Some(query) = self.content.clone() else {
            return Ok(());
        };
        let input = FileOutput {
            display: "-".into(),
            absolute: "-".into(),
            metadata: None,
            eligibility: self
                .options
                .content_options
                .explain_matches
                .then(|| "explicit stdin input".into()),
        };
        let scan = self.content_scan_options();
        let stop_after = self.scan_limit();
        let _ = self.scan_inline(&input, |on_record| {
            query.scan_reader(io::stdin().lock(), &scan, stop_after, on_record)
        })?;
        Ok(())
    }

    fn content_scan_options(&self) -> crate::ContentScanOptions {
        let mut scan = self.options.content_options.scan;
        scan.all_spans = content_spans_needed(&self.options, self.color);
        scan
    }

    fn scan_limit(&self) -> Option<u64> {
        if self.options.content_paths_only() || self.options.quiet {
            Some(1)
        } else {
            self.options
                .limit
                .map(|n| (n as u64).saturating_sub(self.matched))
        }
    }

    pub(super) fn scan_content(&mut self, path: &Path) -> io::Result<ControlFlow<()>> {
        let Some(query) = self.content.clone() else {
            return Ok(ControlFlow::Continue(()));
        };
        if self.stopped {
            return Ok(ControlFlow::Break(()));
        }
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            self.options.cwd.join(path)
        };
        let metadata = if self.options.json_metadata {
            self.metadata_calls += 1;
            match fs::metadata(&absolute) {
                Ok(metadata) if metadata.is_file() => Some(metadata),
                Ok(_) => return Ok(ControlFlow::Continue(())),
                Err(error) => return self.content_error(path, &error),
            }
        } else {
            None
        };
        let eligibility = self.options.content_options.explain_matches.then(|| {
            if let Some(plan) = &self.content_plan {
                match plan.explain_path(path) {
                    Ok(explanation) => {
                        return explanation
                            .checks
                            .iter()
                            .filter_map(|check| {
                                check
                                    .detail
                                    .as_ref()
                                    .map(|detail| format!("{}: {detail}", check.label))
                            })
                            .collect::<Vec<_>>()
                            .join(" | ");
                    }
                    Err(_) => return "selected by traversal; detailed checks unavailable".into(),
                }
            }
            "selected by the compiled query for an explicit root".into()
        });
        let display = self.output_path(path).into_owned();
        let stop_after = self.scan_limit();
        if let Some(workers) = &self.content_workers {
            let receiver = workers.submit(absolute.clone(), stop_after)?;
            self.pending_content.push_back(PendingContent {
                display,
                absolute,
                metadata,
                eligibility,
                receiver,
            });
            if self.pending_content.len() >= self.content_worker_count {
                return self.drain_content();
            }
            return Ok(ControlFlow::Continue(()));
        }
        let scan = self.content_scan_options();
        let preprocess = self.options.content_options.preprocess.clone();
        let decompress = self.options.content_options.decompress;
        let input = FileOutput {
            display,
            absolute,
            metadata,
            eligibility,
        };
        self.scan_inline(&input, |on_record| {
            content_inputs::scan(
                &query,
                &input.absolute,
                &scan,
                stop_after,
                preprocess.as_ref(),
                decompress,
                on_record,
            )
        })
    }

    fn scan_inline(
        &mut self,
        input: &FileOutput,
        scan: impl FnOnce(
            &mut dyn FnMut(ContentRecord<'_>) -> io::Result<ControlFlow<()>>,
        ) -> io::Result<ScanStats>,
    ) -> io::Result<ControlFlow<()>> {
        let mut write_error = None;
        let result = scan(&mut |record| {
            if let Err(error) = self.content_record(input, record) {
                write_error = Some(error);
                return Ok(ControlFlow::Break(()));
            }
            // The scanner enforces the match limit and can still emit trailing context.
            Ok(ControlFlow::Continue(()))
        });
        if let Some(error) = write_error {
            return Err(error);
        }
        match result {
            Ok(stats) => self.complete_content(input, stats),
            Err(error) => self.content_error(&input.display, &error),
        }
    }

    pub(super) fn drain_content(&mut self) -> io::Result<ControlFlow<()>> {
        let Some(job) = self.pending_content.pop_front() else {
            return Ok(ControlFlow::Continue(()));
        };
        let PendingContent {
            display,
            absolute,
            metadata,
            eligibility,
            receiver,
        } = job;
        let input = FileOutput {
            display,
            absolute,
            metadata,
            eligibility,
        };
        loop {
            match receiver.recv() {
                Ok(ContentEvent::Record(record)) => {
                    let record = record.as_borrowed();
                    self.content_record(&input, record)?;
                }
                Ok(ContentEvent::Done(Ok(stats))) => return self.complete_content(&input, stats),
                Ok(ContentEvent::Done(Err(error))) => {
                    return self.content_error(&input.display, &error);
                }
                Err(error) => return Err(io::Error::other(error)),
            }
        }
    }

    fn complete_content(
        &mut self,
        input: &FileOutput,
        stats: ScanStats,
    ) -> io::Result<ControlFlow<()>> {
        self.content_files += 1;
        self.content_bytes = self.content_bytes.saturating_add(stats.bytes_read);
        if stats.binary_skipped {
            self.content_binary_skipped += 1;
            return Ok(self.finish_match());
        }
        if self.options.content_options.per_file_count {
            let mut path = Vec::new();
            write_path(&mut path, false, false, &input.display)?;
            self.writer.write_all(&path[..path.len() - 1])?;
            writeln!(self.writer, ":{}", stats.matched_lines)?;
        } else if self.options.content_paths_only() {
            let has_match = stats.matched_lines > 0;
            if has_match != self.options.content_options.files_without_matches {
                self.matched += 1;
                if !self.options.quiet {
                    if self.options.print0 {
                        write_path(&mut self.writer, false, true, &input.display)?;
                    } else {
                        self.write_content_row(input, None)?;
                    }
                }
            }
        } else if self.options.quiet && stats.matched_lines > 0 {
            self.matched += 1;
        }
        Ok(self.finish_match())
    }

    fn content_error(&mut self, path: &Path, error: &io::Error) -> io::Result<ControlFlow<()>> {
        self.content_errors += 1;
        self.error(path, error)?;
        Ok(self.finish_match())
    }

    fn content_record(&mut self, input: &FileOutput, record: ContentRecord<'_>) -> io::Result<()> {
        if self.options.content_paths_only() || self.options.quiet {
            return Ok(());
        }
        if record.kind == ContentRecordKind::Match {
            self.matched += 1;
        }
        if self.options.count || self.options.content_options.per_file_count {
            return Ok(());
        }
        self.write_content_row(input, Some(record))
    }

    fn write_content_row(
        &mut self,
        input: &FileOutput,
        record: Option<ContentRecord<'_>>,
    ) -> io::Result<()> {
        self.content_format_state.write(
            &mut self.writer,
            self.options.content_format(),
            Row {
                path: &input.display,
                record,
                metadata: input.metadata.as_ref(),
                eligibility: input.eligibility.as_deref(),
            },
            FormatOptions {
                color: self.color,
                hyperlink: self.hyperlink,
                link_path: (input.display != Path::new("-")).then_some(input.absolute.as_path()),
                only_matching: self.options.content_options.only_matching,
                template: self.options.format.as_deref(),
            },
        )
    }

    pub(crate) fn finish_content_output(&mut self, stats: &Stats) -> io::Result<()> {
        if self.content.is_none() {
            return Ok(());
        }
        let summary = ContentSummary {
            matched: self.matched,
            files: self.content_files,
            bytes: self.content_bytes,
            binary: self.content_binary_skipped,
            errors: stats.errors,
            stopped: self.stopped,
            dirs_pruned: stats.dirs_pruned_positive
                + stats.dirs_pruned_exclude
                + stats.dirs_pruned_ignore
                + stats.dirs_pruned_depth
                + stats.dirs_pruned_policy,
        };
        if self.options.content_options.summary
            && self.options.content_format() == ContentFormat::Text
        {
            eprintln!(
                "{} matched · {} inputs · {} bytes · {} binary skipped · {} errors · {} directories pruned{}",
                summary.matched,
                summary.files,
                summary.bytes,
                summary.binary,
                summary.errors,
                summary.dirs_pruned,
                if summary.stopped {
                    " · stopped at limit"
                } else {
                    ""
                }
            );
        }
        self.content_format_state.finish(
            &mut self.writer,
            self.options.content_format(),
            self.options.content_options.summary.then_some(&summary),
        )?;
        self.writer.flush()
    }
}

pub(super) fn content_spans_needed(options: &Options, terminal: bool) -> bool {
    (!options.content_paths_only()
        && !options.count
        && !options.quiet
        && !options.content_options.per_file_count)
        && (options.content_options.only_matching
            || options.content_format() != ContentFormat::Text
            || options.format.is_some()
            || options.color == Decoration::Always
            || terminal && options.color == Decoration::Auto)
}
