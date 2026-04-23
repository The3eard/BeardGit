<!--
  MergeEditor.svelte — IntelliJ-style 3-panel merge conflict resolution editor.

  Layout: Theirs (Incoming) | Result | Ours (Current)
  - Side panels are readonly CodeMirror editors showing theirs/ours content
    with merge decorations (highlights + gutter accept/ignore buttons).
  - Center panel is an editable CodeMirror editor starting with auto-merged
    content, using conflict placeholders for unresolved conflicts.
  - Scroll sync: scrolling the center panel proportionally scrolls side panels.
  - The "Mark Resolved" button writes the final center editor content back
    via the onResolve callback, with conflict marker detection.
-->
<script lang="ts">
  import { EditorView, lineNumbers } from '@codemirror/view';
  import { EditorState, Compartment } from '@codemirror/state';
  import { history, undo } from '@codemirror/commands';
  import { createCodemirrorTheme } from './codemirror-theme';
  import { getLanguageExtensionName, loadLanguageExtension } from './language-support';
  import {
    mergeHighlightExtension,
    conflictLineWidgetExtension,
    setConflictCallbacks,
    mergeDecorationTheme,
    setMergeHighlights,
    type HighlightRange,
  } from './merge-decorations';
  import {
    threeWayDiff,
    buildMergedResult,
    type MergeChunk,
  } from '$lib/utils/three-way-diff';
  import type { ThemeEditorData } from '$lib/types';
  import * as m from '$lib/paraglide/messages';
  import { renderConnectors, getLineRect, type ConnectorPair } from './merge-connectors';
  import ConfirmDialog from '../common/ConfirmDialog.svelte';

  // ---------------------------------------------------------------------------
  // Conflict placeholder
  // ---------------------------------------------------------------------------

  const CONFLICT_PREFIX = '\u25C6 CONFLICT ';

  /** Generate the placeholder string for a given conflict index. */
  function conflictPlaceholderText(index: number): string {
    return `${CONFLICT_PREFIX}${index}`;
  }

  // ---------------------------------------------------------------------------
  // Props
  // ---------------------------------------------------------------------------

  interface Props {
    /** Content from the current branch ("ours"). */
    ours: string;
    /** Content from the incoming branch ("theirs"). */
    theirs: string;
    /** Content from the common ancestor ("base"). */
    base: string;
    /** Filename used for language detection and display. */
    filename: string;
    /** CodeMirror theme data from the TOML theme system. */
    editorTheme?: ThemeEditorData | null;
    /** Whether the UI is in dark mode. */
    isDark?: boolean;
    /** Called with the resolved file content when the user clicks "Mark Resolved". */
    onResolve?: (content: string) => void;
    /** Called when the user cancels conflict resolution. */
    onCancel?: () => void;
  }

  let {
    ours,
    theirs,
    base,
    filename,
    editorTheme = null,
    isDark = true,
    onResolve,
    onCancel,
  }: Props = $props();

  // ---------------------------------------------------------------------------
  // DOM refs
  // ---------------------------------------------------------------------------

  let theirsEl: HTMLDivElement | undefined = $state();
  let resultEl: HTMLDivElement | undefined = $state();
  let oursEl: HTMLDivElement | undefined = $state();
  let leftSvg: SVGSVGElement | undefined = $state();
  let rightSvg: SVGSVGElement | undefined = $state();

  // ---------------------------------------------------------------------------
  // Editor instances
  // ---------------------------------------------------------------------------

  let theirsView: EditorView | undefined;
  let resultView: EditorView | undefined;
  let oursView: EditorView | undefined;

  // ---------------------------------------------------------------------------
  // Merge state
  // ---------------------------------------------------------------------------

  let chunks = $state<MergeChunk[]>([]);
  let resolvedConflicts = $state(new Set<number>());
  let showResolveConfirm = $state(false);
  let showLineNumbers = $state(false);

  // Compartments for toggling line numbers on all 3 editors
  const theirsLineNumComp = new Compartment();
  const resultLineNumComp = new Compartment();
  const oursLineNumComp = new Compartment();

  function toggleLineNumbers() {
    showLineNumbers = !showLineNumbers;
    const ext = showLineNumbers ? lineNumbers() : [];
    theirsView?.dispatch({ effects: theirsLineNumComp.reconfigure(ext) });
    resultView?.dispatch({ effects: resultLineNumComp.reconfigure(ext) });
    oursView?.dispatch({ effects: oursLineNumComp.reconfigure(ext) });
  }

  let totalConflicts = $derived(chunks.filter((c) => c.kind === 'conflict').length);
  let resolvedCount = $derived(resolvedConflicts.size);
  let allResolved = $derived(resolvedCount === totalConflicts);
  let activeConflictIndex = $state<number | null>(null);

  /** Dynamic gap width: wider when many conflicts need more curve space. */
  let gapWidth = $derived(totalConflicts > 4 ? 40 : 24);

  // ---------------------------------------------------------------------------
  // Line arrays (kept in sync with props)
  // ---------------------------------------------------------------------------

  let baseLines: string[] = [];
  let theirsLines: string[] = [];
  let oursLines: string[] = [];

  // ---------------------------------------------------------------------------
  // Highlight computation
  // ---------------------------------------------------------------------------

  /**
   * Compute highlight ranges for a side panel from the current chunks.
   *
   * Colors: green = added lines, purple = conflict lines.
   * After a conflict is resolved, its highlight changes to green.
   */
  function computeSideHighlights(
    side: 'left' | 'right',
    currentChunks: MergeChunk[],
    resolved: Set<number>,
    activeIdx: number | null,
  ): HighlightRange[] {
    const highlights: HighlightRange[] = [];
    let conflictIdx = 0;

    for (const chunk of currentChunks) {
      const range = side === 'left' ? chunk.theirsRange : chunk.oursRange;

      if (chunk.kind === 'theirs_only' && side === 'left' && range.count > 0) {
        highlights.push({
          fromLine: range.start,
          lineCount: range.count,
          kind: 'added',
          conflictIndex: -1,
        });
      } else if (chunk.kind === 'ours_only' && side === 'right' && range.count > 0) {
        highlights.push({
          fromLine: range.start,
          lineCount: range.count,
          kind: 'added',
          conflictIndex: -1,
        });
      } else if (chunk.kind === 'conflict') {
        const idx = conflictIdx++;
        if (range.count > 0) {
          const isActive = idx === activeIdx && !resolved.has(idx);
          highlights.push({
            fromLine: range.start,
            lineCount: range.count,
            kind: resolved.has(idx) ? 'added' : (isActive ? 'conflict-active' : 'conflict'),
            conflictIndex: resolved.has(idx) ? -1 : idx,
          });
        }
      }
    }

    return highlights;
  }

  /**
   * Compute highlight ranges for the center (result) panel.
   * Conflict placeholder lines get blue background.
   */
  function computeCenterHighlights(resultContent: string, activeIdx: number | null): HighlightRange[] {
    const highlights: HighlightRange[] = [];
    const lines = resultContent.split('\n');
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].startsWith(CONFLICT_PREFIX)) {
        const idx = parseInt(lines[i].slice(CONFLICT_PREFIX.length), 10);
        const isActive = !isNaN(idx) && idx === activeIdx;
        highlights.push({
          fromLine: i,
          lineCount: 1,
          kind: isActive ? 'conflict-center-active' : 'conflict-center',
          conflictIndex: -1,
        });
      }
    }
    return highlights;
  }

  // ---------------------------------------------------------------------------
  // Scroll sync
  // ---------------------------------------------------------------------------

  /**
   * Build a line mapping from center (result) lines to side panel lines.
   *
   * For each center line, stores which theirs/ours line corresponds.
   * Used for chunk-aware scroll sync.
   */
  let centerToTheirs: number[] = [];
  let centerToOurs: number[] = [];

  function buildLineMapping(currentChunks: MergeChunk[]) {
    centerToTheirs = [];
    centerToOurs = [];

    let ci = 0; // center line index
    let ti = 0; // theirs line index
    let oi = 0; // ours line index

    for (const chunk of currentChunks) {
      switch (chunk.kind) {
        case 'unchanged': {
          for (let i = 0; i < chunk.baseRange.count; i++) {
            centerToTheirs[ci] = ti;
            centerToOurs[ci] = oi;
            ci++; ti++; oi++;
          }
          break;
        }
        case 'theirs_only': {
          // These lines in center come from theirs
          for (let i = 0; i < chunk.theirsRange.count; i++) {
            centerToTheirs[ci] = ti;
            centerToOurs[ci] = oi; // ours stays at same position
            ci++; ti++;
          }
          break;
        }
        case 'ours_only': {
          // These lines in center come from ours
          for (let i = 0; i < chunk.oursRange.count; i++) {
            centerToTheirs[ci] = ti; // theirs stays at same position
            centerToOurs[ci] = oi;
            ci++; oi++;
          }
          break;
        }
        case 'conflict': {
          // Conflict placeholder = 1 center line maps to the start of both side ranges
          centerToTheirs[ci] = ti;
          centerToOurs[ci] = oi;
          ci++;
          ti += chunk.theirsRange.count;
          oi += chunk.oursRange.count;
          break;
        }
      }
    }
  }

  /**
   * Sync side panels to the center editor using chunk-aware line mapping.
   *
   * Finds the top visible line in the center, looks up the corresponding
   * line in each side panel, and scrolls the side to align.
   */
  function syncScrollFromCenter() {
    if (!resultView || !theirsView || !oursView) return;
    if (centerToTheirs.length === 0) return;

    // Find the top visible line in the center editor
    const topPos = resultView.elementAtHeight(resultView.scrollDOM.scrollTop);
    const topLine = resultView.state.doc.lineAt(topPos.from).number - 1; // 0-based

    // Clamp to mapping range
    const mappedLine = Math.min(topLine, centerToTheirs.length - 1);
    if (mappedLine < 0) return;

    const theirsLine = centerToTheirs[mappedLine] ?? 0;
    const oursLine = centerToOurs[mappedLine] ?? 0;

    // Scroll side panels to the mapped line
    scrollViewToLine(theirsView, theirsLine);
    scrollViewToLine(oursView, oursLine);
  }

  /** Scroll an editor so a 0-based line index is at the top (smooth). */
  function scrollViewToLine(view: EditorView, line0: number) {
    const lineNum = Math.max(1, Math.min(line0 + 1, view.state.doc.lines));
    const pos = view.state.doc.line(lineNum).from;
    const block = view.lineBlockAt(pos);
    view.scrollDOM.scrollTo({ top: block.top, behavior: 'smooth' });
  }

  // ---------------------------------------------------------------------------
  // Accept / ignore handlers
  // ---------------------------------------------------------------------------

  /**
   * Replace a conflict placeholder in the center editor with the given lines,
   * or remove it entirely if lines is empty (ignore).
   */
  function replaceConflictPlaceholder(conflictIndex: number, lines: string[]) {
    if (!resultView) return;

    const doc = resultView.state.doc;
    const placeholder = conflictPlaceholderText(conflictIndex);

    // Find the placeholder line in the document
    for (let lineNum = 1; lineNum <= doc.lines; lineNum++) {
      const line = doc.line(lineNum);
      if (line.text === placeholder) {
        const replacement = lines.join('\n');
        resultView.dispatch({
          changes: { from: line.from, to: line.to, insert: replacement },
        });
        break;
      }
    }

    // Mark as resolved
    resolvedConflicts = new Set([...resolvedConflicts, conflictIndex]);

    // Rebuild highlights for side panels and center
    updateSideHighlights();
    updateCenterHighlights();
    updateConnectors();
  }

  /** Accept a conflict from the theirs (left) side. */
  function handleAcceptTheirs(conflictIndex: number) {
    const chunk = getConflictChunk(conflictIndex);
    if (!chunk) return;

    const lines = theirsLines.slice(
      chunk.theirsRange.start,
      chunk.theirsRange.start + chunk.theirsRange.count,
    );
    replaceConflictPlaceholder(conflictIndex, lines);
  }

  /** Accept a conflict from the ours (right) side. */
  function handleAcceptOurs(conflictIndex: number) {
    const chunk = getConflictChunk(conflictIndex);
    if (!chunk) return;

    const lines = oursLines.slice(
      chunk.oursRange.start,
      chunk.oursRange.start + chunk.oursRange.count,
    );
    replaceConflictPlaceholder(conflictIndex, lines);
  }

  /** Ignore a conflict — remove the placeholder entirely. */
  function handleIgnoreTheirs(conflictIndex: number) {
    replaceConflictPlaceholder(conflictIndex, []);
  }

  /** Ignore a conflict — remove the placeholder entirely. */
  function handleIgnoreOurs(conflictIndex: number) {
    replaceConflictPlaceholder(conflictIndex, []);
  }

  /** Get the Nth conflict chunk (0-based conflict index). */
  function getConflictChunk(conflictIndex: number): MergeChunk | undefined {
    let idx = 0;
    for (const chunk of chunks) {
      if (chunk.kind === 'conflict') {
        if (idx === conflictIndex) return chunk;
        idx++;
      }
    }
    return undefined;
  }

  // ---------------------------------------------------------------------------
  // Side panel highlight updates
  // ---------------------------------------------------------------------------

  /** Dispatch updated highlights to both side panels. */
  function updateSideHighlights() {
    if (!theirsView || !oursView) return;

    const theirsHighlights = computeSideHighlights('left', chunks, resolvedConflicts, activeConflictIndex);
    const oursHighlights = computeSideHighlights('right', chunks, resolvedConflicts, activeConflictIndex);

    theirsView.dispatch({ effects: setMergeHighlights.of(theirsHighlights) });
    oursView.dispatch({ effects: setMergeHighlights.of(oursHighlights) });

  }

  /** Refresh center panel highlights (blue on remaining conflict placeholders). */
  function updateCenterHighlights() {
    if (!resultView) return;
    const content = resultView.state.doc.toString();
    const centerHighlights = computeCenterHighlights(content, activeConflictIndex);
    resultView.dispatch({ effects: setMergeHighlights.of(centerHighlights) });
  }

  import { untrack } from 'svelte';

  // ---------------------------------------------------------------------------
  // SVG connectors
  // ---------------------------------------------------------------------------

  /** Find the line index (0-based) of a conflict placeholder in the result doc. */
  function findPlaceholderLine(conflictIndex: number): number {
    if (!resultView) return -1;
    const doc = resultView.state.doc;
    const target = conflictPlaceholderText(conflictIndex);
    for (let ln = 1; ln <= doc.lines; ln++) {
      if (doc.line(ln).text === target) return ln - 1;
    }
    return -1;
  }

  /** Render SVG bezier connectors between side panels and the center panel. */
  function updateConnectors() {
    if (!theirsView || !resultView || !oursView || !leftSvg || !rightSvg) return;

    // Use the connector gap's bounding rect as reference for Y coordinates
    const leftGapRect = leftSvg.parentElement?.getBoundingClientRect();
    const rightGapRect = rightSvg.parentElement?.getBoundingClientRect();
    if (!leftGapRect || !rightGapRect) return;

    let conflictIdx = 0;
    const leftPairs: ConnectorPair[] = [];
    const rightPairs: ConnectorPair[] = [];

    for (const chunk of chunks) {
      if (chunk.kind !== 'conflict') continue;
      const idx = conflictIdx++;
      const resolved = resolvedConflicts.has(idx);

      const centerLineIdx = findPlaceholderLine(idx);
      if (centerLineIdx < 0 && !resolved) continue;

      const theirsRect = getLineRect(theirsView, chunk.theirsRange.start, chunk.theirsRange.count, leftGapRect.top);
      const oursRect = getLineRect(oursView, chunk.oursRange.start, chunk.oursRange.count, rightGapRect.top);
      const centerRectLeft = centerLineIdx >= 0
        ? getLineRect(resultView, centerLineIdx, 1, leftGapRect.top)
        : { top: 0, bottom: 0 };
      const centerRectRight = centerLineIdx >= 0
        ? getLineRect(resultView, centerLineIdx, 1, rightGapRect.top)
        : { top: 0, bottom: 0 };

      leftPairs.push({ side: theirsRect, center: centerRectLeft, resolved });
      rightPairs.push({ side: oursRect, center: centerRectRight, resolved });
    }

    // Set SVG dimensions explicitly (SVG elements ignore CSS width/height)
    const gapHeight = leftGapRect.height;
    const w = leftGapRect.width;
    leftSvg.setAttribute('width', String(w));
    leftSvg.setAttribute('height', String(gapHeight));
    rightSvg.setAttribute('width', String(w));
    rightSvg.setAttribute('height', String(gapHeight));

    renderConnectors(leftSvg, leftPairs, w, 'left');
    renderConnectors(rightSvg, rightPairs, w, 'right');
  }

  // ---------------------------------------------------------------------------
  // Undo support
  // ---------------------------------------------------------------------------

  /** Undo the last change in the result editor and rescan conflict state. */
  function handleUndo() {
    if (resultView) {
      undo(resultView);
      rescanConflicts();
    }
  }

  /**
   * Rescan the result document for remaining conflict placeholders
   * and rebuild the resolvedConflicts set accordingly.
   */
  function rescanConflicts() {
    if (!resultView) return;
    const doc = resultView.state.doc;
    const found = new Set<number>();
    for (let ln = 1; ln <= doc.lines; ln++) {
      const text = doc.line(ln).text;
      if (text.startsWith(CONFLICT_PREFIX)) {
        const idx = parseInt(text.slice(CONFLICT_PREFIX.length), 10);
        if (!isNaN(idx)) found.add(idx);
      }
    }
    const newResolved = new Set<number>();
    for (let i = 0; i < untrack(() => totalConflicts); i++) {
      if (!found.has(i)) newResolved.add(i);
    }
    untrack(() => { resolvedConflicts = newResolved; });
    updateSideHighlights();
    updateCenterHighlights();
    updateConnectors();
  }

  // ---------------------------------------------------------------------------
  // Editor initialization
  // ---------------------------------------------------------------------------

  /** Destroy all existing editors and create 3 fresh ones for the merge layout. */
  async function initEditors() {

    theirsView?.destroy();
    resultView?.destroy();
    oursView?.destroy();
    theirsView = undefined;
    resultView = undefined;
    oursView = undefined;

    // Split content into lines
    baseLines = base === '' ? [] : base.split('\n');
    theirsLines = theirs === '' ? [] : theirs.split('\n');
    oursLines = ours === '' ? [] : ours.split('\n');

    // Compute 3-way diff chunks (untracked to avoid re-triggering effects)
    const newChunks = threeWayDiff(base, theirs, ours);
    untrack(() => {
      chunks = newChunks;
      resolvedConflicts = new Set();
    });

    // Build auto-merged result with placeholders for conflicts
    const mergedContent = buildMergedResult(
      newChunks,
      baseLines,
      theirsLines,
      oursLines,
      (i) => conflictPlaceholderText(i),
    );

    // Compute initial highlights (use newChunks directly to avoid reading $state)
    const theirsHighlights = computeSideHighlights('left', newChunks, new Set(), null);
    const oursHighlights = computeSideHighlights('right', newChunks, new Set(), null);

    // Load language and theme extensions

    const langName = getLanguageExtensionName(filename);
    const langExt = langName ? await loadLanguageExtension(langName) : null;


    // After await, check if this init call is still valid (not superseded by
    // a new effect run that destroyed the editors).
    if (!theirsEl || !resultEl || !oursEl) return;

    const theme = createCodemirrorTheme(editorTheme, isDark);

    // --- Theirs (Incoming) --- readonly side panel
    const lineNumExt = showLineNumbers ? lineNumbers() : [];
    const theirsExts = [
      theme,
      theirsLineNumComp.of(lineNumExt),
      EditorState.readOnly.of(true),
      EditorView.lineWrapping,
      mergeHighlightExtension(),
      mergeDecorationTheme(),
    ];
    if (langExt) theirsExts.push(langExt);

    theirsView = new EditorView({
      state: EditorState.create({ doc: theirs, extensions: theirsExts }),
      parent: theirsEl,
    });

    // --- Result --- editable center panel with blue conflict placeholders
    const resultExts = [
      theme,
      resultLineNumComp.of(lineNumExt),
      EditorView.lineWrapping,
      history(),
      mergeHighlightExtension(),
      conflictLineWidgetExtension(),
      mergeDecorationTheme(),
    ];
    if (langExt) resultExts.push(langExt);

    resultView = new EditorView({
      state: EditorState.create({ doc: mergedContent, extensions: resultExts }),
      parent: resultEl,
    });

    // Highlight conflict placeholder lines in center (blue) and set up widget callbacks
    const centerHighlights = computeCenterHighlights(mergedContent, null);
    resultView.dispatch({
      effects: [
        setMergeHighlights.of(centerHighlights),
        setConflictCallbacks.of({
          acceptTheirs: handleAcceptTheirs,
          acceptOurs: handleAcceptOurs,
          ignoreTheirs: handleIgnoreTheirs,
          ignoreOurs: handleIgnoreOurs,
        }),
      ],
    });

    // --- Ours (Current) --- readonly side panel
    const oursExts = [
      theme,
      oursLineNumComp.of(lineNumExt),
      EditorState.readOnly.of(true),
      EditorView.lineWrapping,
      mergeHighlightExtension(),
      mergeDecorationTheme(),
    ];
    if (langExt) oursExts.push(langExt);

    oursView = new EditorView({
      state: EditorState.create({ doc: ours, extensions: oursExts }),
      parent: oursEl,
    });

    // Dispatch initial highlights to side panels
    theirsView.dispatch({ effects: setMergeHighlights.of(theirsHighlights) });
    oursView.dispatch({ effects: setMergeHighlights.of(oursHighlights) });


    // Build line mapping for chunk-aware scroll sync
    buildLineMapping(newChunks);

    // --- Scroll sync: center drives both side panels ---
    resultView.scrollDOM.addEventListener('scroll', () => {
      syncScrollFromCenter();
      updateConnectors();
    });

    // Update connectors when side panels scroll (during smooth animation)
    theirsView.scrollDOM.addEventListener('scroll', () => updateConnectors());
    oursView.scrollDOM.addEventListener('scroll', () => updateConnectors());

    // Intercept wheel events on side panels → scroll center instead
    function redirectWheel(e: WheelEvent) {
      e.preventDefault();
      if (resultView) {
        resultView.scrollDOM.scrollTop += e.deltaY;
      }
    }
    theirsView.scrollDOM.addEventListener('wheel', redirectWheel, { passive: false });
    oursView.scrollDOM.addEventListener('wheel', redirectWheel, { passive: false });

    // Initial connector render (after DOM layout settles)
    requestAnimationFrame(() => {
      updateConnectors();
    });
  }

  // ---------------------------------------------------------------------------
  // Navigation
  // ---------------------------------------------------------------------------

  /** Scroll the center editor to the next unresolved conflict placeholder. */
  function handleNextConflict() {
    if (!resultView) return;
    scrollToConflict('forward');
  }

  /** Scroll the center editor to the previous unresolved conflict placeholder. */
  function handlePrevConflict() {
    if (!resultView) return;
    scrollToConflict('backward');
  }

  /**
   * Find the next/previous unresolved conflict and scroll all 3 panels
   * so the conflict regions align side by side.
   */
  function scrollToConflict(direction: 'forward' | 'backward') {
    if (!resultView || !theirsView || !oursView) return;

    // Build list of unresolved conflict indices and their chunks
    const unresolvedConflicts: { idx: number; chunk: MergeChunk; placeholderLine: number }[] = [];
    let conflictIdx = 0;
    for (const chunk of chunks) {
      if (chunk.kind !== 'conflict') continue;
      const idx = conflictIdx++;
      if (resolvedConflicts.has(idx)) continue;
      const pl = findPlaceholderLine(idx);
      if (pl >= 0) unresolvedConflicts.push({ idx, chunk, placeholderLine: pl });
    }

    if (unresolvedConflicts.length === 0) return;

    // Find current position in the center editor
    const doc = resultView.state.doc;
    const cursorLine = doc.lineAt(resultView.state.selection.main.head).number;

    // Find target conflict
    let target: typeof unresolvedConflicts[0];
    if (direction === 'forward') {
      const next = unresolvedConflicts.find(c => c.placeholderLine + 1 > cursorLine);
      target = next ?? unresolvedConflicts[0];
    } else {
      const prev = [...unresolvedConflicts].reverse().find(c => c.placeholderLine + 1 < cursorLine);
      target = prev ?? unresolvedConflicts[unresolvedConflicts.length - 1];
    }

    // Scroll center editor to the conflict placeholder
    const centerLine = doc.line(target.placeholderLine + 1);
    resultView.dispatch({
      selection: { anchor: centerLine.from },
      effects: EditorView.scrollIntoView(centerLine.from, { y: 'center' }),
    });

    // Scroll side panels to align their conflict regions
    scrollSideToLine(theirsView, target.chunk.theirsRange.start);
    scrollSideToLine(oursView, target.chunk.oursRange.start);

    // Highlight the active conflict
    activeConflictIndex = target.idx;
    updateSideHighlights();
    updateCenterHighlights();

    // Update connectors after scroll settles
    requestAnimationFrame(() => updateConnectors());
  }

  /** Scroll a side panel so a given 0-based line is centered vertically. */
  function scrollSideToLine(view: EditorView, line0: number) {
    const lineNum = Math.min(line0 + 1, view.state.doc.lines);
    const pos = view.state.doc.line(lineNum).from;
    view.dispatch({
      effects: EditorView.scrollIntoView(pos, { y: 'center' }),
    });
  }

  // ---------------------------------------------------------------------------
  // Resolve handling
  // ---------------------------------------------------------------------------

  /** Returns true if the content contains any git conflict markers. */
  function hasConflictMarkers(content: string): boolean {
    return /^<{7}[\s]|^={7}\s*$|^>{7}[\s]/m.test(content.replace(/\r\n/g, '\n'));
  }

  /** Returns true if the content still has unresolved conflict placeholders. */
  function hasConflictPlaceholders(content: string): boolean {
    return content.includes(CONFLICT_PREFIX);
  }

  /** Handle resolve button click — show confirmation if conflict markers remain. */
  function handleResolveClick() {
    if (!resultView) return;
    const content = resultView.state.doc.toString();
    if (hasConflictMarkers(content) || hasConflictPlaceholders(content)) {
      showResolveConfirm = true;
    } else {
      onResolve?.(content);
    }
  }

  /** Called when the user confirms resolving despite remaining conflict markers. */
  function confirmResolve() {
    if (resultView && onResolve) {
      onResolve(resultView.state.doc.toString());
    }
    showResolveConfirm = false;
  }

  // ---------------------------------------------------------------------------
  // Effect: mount/unmount editors
  // ---------------------------------------------------------------------------

  /**
   * Mount/unmount editors when the container elements or any
   * content / theme props change.
   */
  $effect(() => {

    // Read reactive deps so the effect re-runs on change.
    const _ours = ours;
    const _theirs = theirs;
    const _base = base;
    const _file = filename;
    const _theme = editorTheme;
    const _dark = isDark;
    // DOM refs must be reactive so the effect re-runs after mount.
    const _thEl = theirsEl;
    const _rEl = resultEl;
    const _oEl = oursEl;

    if (!_thEl || !_rEl || !_oEl) return;

    initEditors();

    return () => {
      theirsView?.destroy();
      resultView?.destroy();
      oursView?.destroy();
      theirsView = undefined;
      resultView = undefined;
      oursView = undefined;
    };
  });
</script>

<svelte:window onresize={() => updateConnectors()} />

<div class="merge-editor-wrapper">
  <div class="merge-toolbar">
    <span class="merge-filename">{filename}</span>
    <div class="merge-actions">
      <button
        class="merge-btn nav"
        onclick={handlePrevConflict}
        title={m.merge_prev_conflict()}
      >
        <span class="nf">{"\uF062"}</span>
      </button>
      <button
        class="merge-btn nav"
        onclick={handleNextConflict}
        title={m.merge_next_conflict()}
      >
        <span class="nf">{"\uF063"}</span>
      </button>
      <button class="merge-btn nav" onclick={handleUndo} title={m.merge_undo()}>
        <span class="nf">{"\uF2EA"}</span>
      </button>
      <button
        class="merge-btn nav"
        class:active={showLineNumbers}
        onclick={toggleLineNumbers}
        title="Toggle line numbers"
      >
        <span class="nf">{"\uF292"}</span>
      </button>
      <span class="conflict-counter">
        {m.merge_conflicts_counter({ resolved: String(resolvedCount), total: String(totalConflicts) })}
      </span>
      <button
        class="merge-btn resolve"
        class:enabled={allResolved || totalConflicts === 0}
        onclick={handleResolveClick}
        disabled={totalConflicts > 0 && !allResolved}
      >
        {m.merge_mark_resolved()}
      </button>
      {#if onCancel}
        <button class="merge-btn cancel" onclick={onCancel}>{m.merge_cancel()}</button>
      {/if}
    </div>
  </div>
  <div class="merge-panels">
    <div class="panel-headers">
      <div class="panel-header">{m.merge_panel_theirs()}</div>
      <div class="panel-header-gap" style="width: {gapWidth}px"></div>
      <div class="panel-header panel-header-center">{m.merge_panel_result()}</div>
      <div class="panel-header-gap" style="width: {gapWidth}px"></div>
      <div class="panel-header">{m.merge_panel_ours()}</div>
    </div>
    <div class="panel-editors">
      <div class="panel-editor" bind:this={theirsEl}></div>
      <div class="connector-gap" style="width: {gapWidth}px">
        <svg bind:this={leftSvg} class="connector-svg"></svg>
      </div>
      <div class="panel-editor panel-editor-center" bind:this={resultEl}></div>
      <div class="connector-gap" style="width: {gapWidth}px">
        <svg bind:this={rightSvg} class="connector-svg"></svg>
      </div>
      <div class="panel-editor" bind:this={oursEl}></div>
    </div>
  </div>
</div>

{#if showResolveConfirm}
  <ConfirmDialog
    title={m.merge_resolve_confirm_title()}
    message={m.merge_resolve_confirm_message()}
    confirmLabel={m.merge_mark_resolved()}
    onConfirm={confirmResolve}
    onCancel={() => { showResolveConfirm = false; }}
  />
{/if}

<style>
  .merge-editor-wrapper {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .merge-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    gap: 8px;
    flex-shrink: 0;
  }

  .merge-filename {
    font-family: var(--font-mono);
    color: var(--accent-blue);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .merge-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .merge-btn {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 11px;
    padding: 3px 10px;
    border-radius: 4px;
    white-space: nowrap;
  }

  .merge-btn:hover {
    background: var(--selection);
  }

  .merge-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .merge-btn.nav {
    font-size: 11px;
    padding: 3px 6px;
  }

  .merge-btn.nav .nf {
    font-family: var(--font-icons);
  }

  .merge-btn.nav.active {
    background: color-mix(in srgb, var(--accent-blue) 15%, transparent);
    border-color: var(--accent-blue);
    color: var(--accent-blue);
  }

  .merge-btn.resolve {
    background: color-mix(in srgb, var(--text-secondary) 20%, transparent);
    color: var(--text-secondary);
    border-color: var(--border);
    font-weight: 600;
  }

  .merge-btn.resolve:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .merge-btn.resolve.enabled {
    background: var(--accent-green);
    color: var(--bg-primary);
    border-color: var(--accent-green);
    cursor: pointer;
  }

  .merge-btn.resolve.enabled:hover {
    opacity: 0.9;
  }

  .merge-btn.cancel {
    color: var(--accent-red);
    border-color: color-mix(in srgb, var(--accent-red) 30%, transparent);
    background: var(--overlay-accent-red);
  }

  .merge-btn.cancel:hover {
    background: color-mix(in srgb, var(--accent-red) 20%, transparent);
  }

  .conflict-counter {
    color: var(--accent-orange);
    font-size: 11px;
    font-weight: 600;
    padding: 0 4px;
  }

  .merge-panels {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .panel-headers {
    display: flex;
    flex-shrink: 0;
    border-bottom: 1px solid var(--border);
  }

  .panel-header {
    flex: 1;
    padding: 4px 10px;
    background: var(--bg-secondary);
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .panel-header-gap {
    width: 24px;
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .panel-header-center {
    flex: 1.2;
    color: var(--accent-blue);
  }

  .panel-editors {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .panel-editor {
    flex: 1;
    overflow: hidden;
    /* Create stacking context so editor gutters don't escape into connector gaps */
    isolation: isolate;
  }

  .panel-editor-center {
    flex: 1.2;
  }

  .connector-gap {
    width: 24px;
    flex-shrink: 0;
    position: relative;
    z-index: 2;
    background: var(--bg-primary);
  }

  .connector-svg {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: visible;
  }

  .panel-editor :global(.cm-editor) {
    height: 100%;
  }

  .panel-editor :global(.cm-scroller) {
    overflow: auto;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: 12px;
    line-height: 1.5;
  }
</style>
