<!--
  MergeEditor.svelte — IntelliJ-style 3-panel merge conflict resolution editor.

  Layout: Theirs (Incoming) | Result | Ours (Current)
  - Side panels are readonly CodeMirror editors showing theirs/ours content.
    Every conflict chunk carries a header widget with accept / discard
    buttons for that side, so the controls sit next to the lines they act on.
  - Center panel is an editable CodeMirror editor starting with the
    auto-merged content. Each unresolved conflict is one placeholder line,
    rendered as a widget with the same actions for both sides.
  - A conflict is resolved once both sides are decided. Accepting a side
    inserts its lines above the placeholder, so accepting both keeps theirs
    followed by ours; discarding both removes the block. The per-side state
    lives in the placeholder text, so undo in the result editor also undoes
    the decision (see merge-placeholder.ts).
  - Scroll: every panel scrolls natively. Scrolling one panel moves the
    other two to the same fraction of the same chunk, so a 200-line block on
    one side scrolls through against a one-line placeholder on the other
    instead of being skipped (see merge-scroll-sync.ts). The link button in
    the toolbar turns the coupling off.
  - "Mark Resolved" hands the result content to `onResolve`, after a
    confirmation if conflict markers or placeholders remain.
-->
<script lang="ts">
  import { EditorView, lineNumbers } from '@codemirror/view';
  import { EditorState, Compartment } from '@codemirror/state';
  import { history, undo } from '@codemirror/commands';
  import { untrack, onDestroy } from 'svelte';
  import { createCodemirrorTheme } from './codemirror-theme';
  import { getLanguageExtensionName, loadLanguageExtension } from './language-support';
  import {
    mergeHighlightExtension,
    conflictLineWidgetExtension,
    sideConflictWidgetExtension,
    setConflictCallbacks,
    setSideConflicts,
    setActiveConflict,
    mergeDecorationTheme,
    setMergeHighlights,
    type HighlightRange,
    type SideConflict,
  } from './merge-decorations';
  import {
    decideSide,
    findPlaceholders,
    formatPlaceholder,
    hasPlaceholders,
    pendingMarker,
    type ConflictMarker,
    type ConflictSide,
    type SideDecision,
  } from './merge-placeholder';
  import {
    centerChunkStartLines,
    mapScrollOffset,
    sideChunkStartLines,
  } from './merge-scroll-sync';
  import {
    threeWayDiff,
    buildMergedResult,
    type MergeChunk,
  } from '$lib/utils/three-way-diff';
  import * as m from '$lib/paraglide/messages';
  import { renderConnectors, type ConnectorPair, type RegionRect } from './merge-connectors';
  import ConfirmDialog from '../common/ConfirmDialog.svelte';
  import { Button, IconButton } from '$lib/components/ui';

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
  /** Indices of conflicts whose placeholder is gone from the result. */
  let resolvedConflicts = $state(new Set<number>());
  let showResolveConfirm = $state(false);
  let showLineNumbers = $state(false);
  let scrollLinked = $state(true);
  let activeConflictIndex = $state<number | null>(null);

  /** Current placeholder state per conflict index, rescanned from the result. */
  let markers = new Map<number, ConflictMarker>();
  /**
   * Final state of conflicts whose placeholder was removed by the second
   * decision. The placeholder carried the state until then; this keeps it
   * for the side badges. Dropped again if undo brings the placeholder back.
   */
  let finalDecisions = new Map<number, ConflictMarker>();

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

  function toggleScrollLink() {
    scrollLinked = !scrollLinked;
    if (scrollLinked && leader) syncFrom(leader);
  }

  let totalConflicts = $derived(chunks.filter((c) => c.kind === 'conflict').length);
  let resolvedCount = $derived(resolvedConflicts.size);
  let allResolved = $derived(resolvedCount === totalConflicts);

  /** Dynamic gap width: wider when many conflicts need more curve space. */
  let gapWidth = $derived(totalConflicts > 4 ? 40 : 24);

  // ---------------------------------------------------------------------------
  // Line arrays (kept in sync with props)
  // ---------------------------------------------------------------------------

  let baseLines: string[] = [];
  let theirsLines: string[] = [];
  let oursLines: string[] = [];

  /** Conflict chunks in order; position in this array is the conflict index. */
  let conflictChunks: MergeChunk[] = [];

  function decisionOf(index: number, side: ConflictSide): SideDecision {
    const marker = markers.get(index) ?? finalDecisions.get(index);
    if (marker) return marker[side];
    // Placeholder gone with no record of how: treat as decided.
    return resolvedConflicts.has(index) ? 'accepted' : 'pending';
  }

  // ---------------------------------------------------------------------------
  // Highlight computation
  // ---------------------------------------------------------------------------

  /**
   * Compute highlight ranges for a side panel.
   *
   * Green for lines this side adds outright and for an accepted conflict
   * side, red for a discarded one, purple while the conflict is pending.
   */
  function computeSideHighlights(side: ConflictSide): HighlightRange[] {
    const highlights: HighlightRange[] = [];
    let conflictIdx = 0;

    for (const chunk of chunks) {
      const range = side === 'theirs' ? chunk.theirsRange : chunk.oursRange;

      if (chunk.kind === 'theirs_only' && side === 'theirs' && range.count > 0) {
        highlights.push({ fromLine: range.start, lineCount: range.count, kind: 'added', conflictIndex: -1 });
      } else if (chunk.kind === 'ours_only' && side === 'ours' && range.count > 0) {
        highlights.push({ fromLine: range.start, lineCount: range.count, kind: 'added', conflictIndex: -1 });
      } else if (chunk.kind === 'conflict') {
        const idx = conflictIdx++;
        if (range.count === 0) continue;
        const decision = decisionOf(idx, side);
        const kind: HighlightRange['kind'] =
          decision === 'accepted' ? 'added'
          : decision === 'discarded' ? 'removed'
          : idx === activeConflictIndex ? 'conflict-active'
          : 'conflict';
        highlights.push({ fromLine: range.start, lineCount: range.count, kind, conflictIndex: idx });
      }
    }

    return highlights;
  }

  /** Side-panel conflict widgets, one per conflict chunk. */
  function computeSideConflicts(side: ConflictSide): SideConflict[] {
    return conflictChunks.map((chunk, index) => {
      const range = side === 'theirs' ? chunk.theirsRange : chunk.oursRange;
      return {
        index,
        fromLine: range.start,
        lineCount: range.count,
        decision: decisionOf(index, side),
        active: index === activeConflictIndex,
      };
    });
  }

  /** Center highlights: the placeholder lines still in the result. */
  function computeCenterHighlights(): HighlightRange[] {
    if (!resultView) return [];
    return findPlaceholders(resultView.state.doc).map((hit) => ({
      fromLine: hit.lineNumber - 1,
      lineCount: 1,
      kind: hit.marker.index === activeConflictIndex ? 'conflict-center-active' : 'conflict-center',
      conflictIndex: hit.marker.index,
    }));
  }

  // ---------------------------------------------------------------------------
  // Derived-state refresh
  // ---------------------------------------------------------------------------

  /**
   * Rescan the result document and push highlights, side widgets and
   * connectors to all panels. Runs after every result change (including
   * undo), so the document is the single source of truth.
   */
  function refreshDerived() {
    if (!resultView || !theirsView || !oursView) return;

    markers = new Map();
    for (const hit of findPlaceholders(resultView.state.doc)) {
      markers.set(hit.marker.index, hit.marker);
      finalDecisions.delete(hit.marker.index);
    }
    const resolved = new Set<number>();
    for (let i = 0; i < conflictChunks.length; i++) {
      if (!markers.has(i)) resolved.add(i);
    }
    resolvedConflicts = resolved;
    if (activeConflictIndex !== null && resolved.has(activeConflictIndex)) {
      activeConflictIndex = null;
    }

    theirsView.dispatch({
      effects: [
        setMergeHighlights.of(computeSideHighlights('theirs')),
        setSideConflicts.of({ side: 'theirs', conflicts: computeSideConflicts('theirs') }),
      ],
    });
    oursView.dispatch({
      effects: [
        setMergeHighlights.of(computeSideHighlights('ours')),
        setSideConflicts.of({ side: 'ours', conflicts: computeSideConflicts('ours') }),
      ],
    });
    resultView.dispatch({
      effects: [
        setMergeHighlights.of(computeCenterHighlights()),
        setActiveConflict.of(activeConflictIndex),
      ],
    });
    scheduleConnectors();
    scheduleSticky();
    scheduleResync();
  }

  let refreshQueued = false;
  /** Defer a refresh past the current CodeMirror update cycle. */
  function scheduleRefresh() {
    if (refreshQueued) return;
    refreshQueued = true;
    queueMicrotask(() => {
      refreshQueued = false;
      refreshDerived();
    });
  }

  // ---------------------------------------------------------------------------
  // Decisions
  // ---------------------------------------------------------------------------

  /** Lines a side contributes to a conflict (empty when it deleted them). */
  function sideLinesFor(index: number, side: ConflictSide): string[] {
    const chunk = conflictChunks[index];
    if (!chunk) return [];
    const range = side === 'theirs' ? chunk.theirsRange : chunk.oursRange;
    const source = side === 'theirs' ? theirsLines : oursLines;
    return source.slice(range.start, range.start + range.count);
  }

  /** Record a decision for one side of a conflict in the result document. */
  function decide(index: number, side: ConflictSide, decision: 'accepted' | 'discarded') {
    if (!resultView) return;
    const before = markers.get(index);
    const changes = decideSide(resultView.state.doc, index, side, decision, sideLinesFor(index, side));
    if (!changes) return;
    if (before) {
      const after: ConflictMarker = { ...before, [side]: decision };
      if (after.theirs !== 'pending' && after.ours !== 'pending') finalDecisions.set(index, after);
    }
    activeConflictIndex = index;
    resultView.dispatch({ changes, userEvent: 'merge.decide' });
  }

  function activate(index: number) {
    if (activeConflictIndex === index) return;
    activeConflictIndex = index;
    refreshDerived();
  }

  // ---------------------------------------------------------------------------
  // Scroll sync
  // ---------------------------------------------------------------------------

  /**
   * Character offsets in the result document where each chunk starts, plus
   * the document end. Mapped through every change so accepted lines grow the
   * chunk they belong to and the mapping never goes stale.
   */
  let centerChunkPositions: number[] = [];
  let theirsChunkLines: number[] = [];
  let oursChunkLines: number[] = [];

  /** Scroll offsets we set ourselves, so their scroll events are not echoed. */
  const expectedScroll = new Map<EditorView, number>();

  /**
   * The panel the user interacted with last. Only its scrolling drives the
   * others: a layout change in the result (a widget growing after a decision)
   * fires a scroll event there too, and letting it lead would snap the side
   * panels out of the block the user is reading.
   */
  let leader: EditorView | undefined;

  /** Pixel top of every chunk in a side panel, relative to the document. */
  function sideChunkTops(view: EditorView, startLines: number[]): number[] {
    const doc = view.state.doc;
    return startLines.map((line0) =>
      line0 >= doc.lines
        ? view.lineBlockAt(doc.length).bottom
        : view.lineBlockAt(doc.line(line0 + 1).from).top,
    );
  }

  /** Pixel top of every chunk in the result panel, relative to the document. */
  function centerChunkTops(view: EditorView): number[] {
    const last = centerChunkPositions.length - 1;
    return centerChunkPositions.map((pos, i) =>
      i === last ? view.lineBlockAt(view.state.doc.length).bottom : view.lineBlockAt(pos).top,
    );
  }

  function chunkTopsFor(view: EditorView): number[] {
    if (view === resultView) return centerChunkTops(view);
    if (view === theirsView) return sideChunkTops(view, theirsChunkLines);
    return sideChunkTops(view, oursChunkLines);
  }

  function setScrollTop(view: EditorView, top: number) {
    const el = view.scrollDOM;
    const before = el.scrollTop;
    el.scrollTop = top;
    if (el.scrollTop !== before) expectedScroll.set(view, el.scrollTop);
  }

  /** Align the other two panels with `source`'s viewport. */
  function syncFrom(source: EditorView) {
    if (!theirsView || !resultView || !oursView || centerChunkPositions.length === 0) return;
    const srcTops = chunkTopsFor(source);
    const y = source.scrollDOM.scrollTop - source.documentPadding.top;
    for (const target of [theirsView, resultView, oursView]) {
      if (target === source) continue;
      const mapped = mapScrollOffset(srcTops, chunkTopsFor(target), y);
      setScrollTop(target, Math.max(0, mapped + target.documentPadding.top));
    }
  }

  function onPanelScroll(view: EditorView) {
    scheduleConnectors();
    scheduleSticky();
    const expected = expectedScroll.get(view);
    if (expected !== undefined) {
      expectedScroll.delete(view);
      if (Math.abs(expected - view.scrollDOM.scrollTop) < 1.5) return;
    }
    if (scrollLinked && view === leader) syncFrom(view);
  }

  /** Re-align the followers with the leader once layout has settled. */
  function scheduleResync() {
    requestAnimationFrame(() => {
      if (scrollLinked && leader) syncFrom(leader);
    });
  }

  // ---------------------------------------------------------------------------
  // Sticky conflict header
  // ---------------------------------------------------------------------------

  /**
   * The conflict whose header has scrolled off the top of a side panel while
   * its lines still fill the viewport. Its actions are repeated in a bar
   * pinned to the panel's top edge so a long block can be decided from
   * anywhere inside it.
   */
  let stickyTheirs = $state<SideConflict | null>(null);
  let stickyOurs = $state<SideConflict | null>(null);

  function computeSticky(view: EditorView, side: ConflictSide): SideConflict | null {
    const doc = view.state.doc;
    const viewportTop = view.scrollDOM.scrollTop - view.documentPadding.top;
    for (const conflict of computeSideConflicts(side)) {
      if (conflict.lineCount === 0 || conflict.fromLine >= doc.lines) continue;
      const first = view.lineBlockAt(doc.line(conflict.fromLine + 1).from);
      const lastLine = Math.min(conflict.fromLine + conflict.lineCount, doc.lines);
      const bottom = view.lineBlockAt(doc.line(lastLine).from).bottom;
      // `first.top` includes the header widget; the first text line starts
      // where the widget ends, which is what must have left the viewport.
      const headerBottom = first.top + (first.height - view.defaultLineHeight);
      if (headerBottom < viewportTop && bottom > viewportTop + view.defaultLineHeight) return conflict;
    }
    return null;
  }

  let stickyQueued = false;
  function scheduleSticky() {
    if (stickyQueued) return;
    stickyQueued = true;
    requestAnimationFrame(() => {
      stickyQueued = false;
      if (!theirsView || !oursView) return;
      stickyTheirs = computeSticky(theirsView, 'theirs');
      stickyOurs = computeSticky(oursView, 'ours');
    });
  }

  // ---------------------------------------------------------------------------
  // SVG connectors
  // ---------------------------------------------------------------------------

  /** Screen-space rect of a line range, relative to `refTop`, on or off screen. */
  function blockRect(view: EditorView, fromLine: number, lineCount: number, refTop: number): RegionRect {
    const doc = view.state.doc;
    const first = Math.min(fromLine + 1, doc.lines);
    const top = fromLine >= doc.lines
      ? view.lineBlockAt(doc.length).bottom
      : view.lineBlockAt(doc.line(first).from).top;
    const bottom = lineCount === 0
      ? top
      : view.lineBlockAt(doc.line(Math.min(fromLine + lineCount, doc.lines)).from).bottom;
    return { top: top + view.documentTop - refTop, bottom: bottom + view.documentTop - refTop };
  }

  /** Screen-space rect of a result chunk, from its start to the next chunk's. */
  function centerChunkRect(view: EditorView, chunkIdx: number, refTop: number): RegionRect {
    const from = centerChunkPositions[chunkIdx];
    const to = centerChunkPositions[chunkIdx + 1];
    const top = view.lineBlockAt(from).top;
    const bottom = to > from ? view.lineBlockAt(to - 1).bottom : top;
    return { top: top + view.documentTop - refTop, bottom: bottom + view.documentTop - refTop };
  }

  let connectorsQueued = false;
  function scheduleConnectors() {
    if (connectorsQueued) return;
    connectorsQueued = true;
    requestAnimationFrame(() => {
      connectorsQueued = false;
      updateConnectors();
    });
  }

  /** Render SVG bezier connectors between side panels and the center panel. */
  function updateConnectors() {
    if (!theirsView || !resultView || !oursView || !leftSvg || !rightSvg) return;

    const leftGapRect = leftSvg.parentElement?.getBoundingClientRect();
    const rightGapRect = rightSvg.parentElement?.getBoundingClientRect();
    if (!leftGapRect || !rightGapRect) return;

    const leftPairs: ConnectorPair[] = [];
    const rightPairs: ConnectorPair[] = [];
    let conflictIdx = 0;

    chunks.forEach((chunk, chunkIdx) => {
      if (chunk.kind !== 'conflict') return;
      const idx = conflictIdx++;
      const resolved = resolvedConflicts.has(idx);
      if (resolved || chunkIdx + 1 >= centerChunkPositions.length) return;

      const theirsRect = blockRect(theirsView!, chunk.theirsRange.start, chunk.theirsRange.count, leftGapRect.top);
      const oursRect = blockRect(oursView!, chunk.oursRange.start, chunk.oursRange.count, rightGapRect.top);
      leftPairs.push({ side: theirsRect, center: centerChunkRect(resultView!, chunkIdx, leftGapRect.top), resolved, active: idx === activeConflictIndex });
      rightPairs.push({ side: oursRect, center: centerChunkRect(resultView!, chunkIdx, rightGapRect.top), resolved, active: idx === activeConflictIndex });
    });

    const h = leftGapRect.height;
    const w = leftGapRect.width;
    leftSvg.setAttribute('width', String(w));
    leftSvg.setAttribute('height', String(h));
    rightSvg.setAttribute('width', String(w));
    rightSvg.setAttribute('height', String(h));

    renderConnectors(leftSvg, leftPairs, w, h, 'left');
    renderConnectors(rightSvg, rightPairs, w, h, 'right');
  }

  // ---------------------------------------------------------------------------
  // Undo support
  // ---------------------------------------------------------------------------

  /** Undo the last change in the result editor; the update listener rescans. */
  function handleUndo() {
    if (resultView) undo(resultView);
  }

  // ---------------------------------------------------------------------------
  // Editor initialization
  // ---------------------------------------------------------------------------

  // Inputs the editors were last built from. A theme / dark-mode flip
  // re-fires the mount effect, but rebuilding would reset the merged result —
  // wiping the user's in-progress resolution. We only rebuild when the merge
  // inputs themselves change.
  let mountedOurs: string | undefined;
  let mountedTheirs: string | undefined;
  let mountedBase: string | undefined;
  let mountedFile: string | undefined;

  /** Destroy all existing editors and create 3 fresh ones for the merge layout. */
  async function initEditors() {
    theirsView?.destroy();
    resultView?.destroy();
    oursView?.destroy();
    theirsView = undefined;
    resultView = undefined;
    oursView = undefined;
    expectedScroll.clear();

    baseLines = base === '' ? [] : base.split('\n');
    theirsLines = theirs === '' ? [] : theirs.split('\n');
    oursLines = ours === '' ? [] : ours.split('\n');

    const newChunks = threeWayDiff(base, theirs, ours);
    conflictChunks = newChunks.filter((c) => c.kind === 'conflict');
    markers = new Map();
    finalDecisions = new Map();
    untrack(() => {
      chunks = newChunks;
      resolvedConflicts = new Set();
      activeConflictIndex = null;
    });

    const mergedContent = buildMergedResult(
      newChunks,
      baseLines,
      theirsLines,
      oursLines,
      (i) => formatPlaceholder(pendingMarker(i)),
    );

    const langName = getLanguageExtensionName(filename);
    const langExt = langName ? await loadLanguageExtension(langName) : null;

    // After await, check if this init call is still valid (not superseded by
    // a new effect run that destroyed the editors).
    if (!theirsEl || !resultEl || !oursEl) return;

    const theme = createCodemirrorTheme(isDark);
    const lineNumExt = showLineNumbers ? lineNumbers() : [];

    const sideExts = (comp: Compartment) => {
      const exts = [
        theme,
        comp.of(lineNumExt),
        EditorState.readOnly.of(true),
        EditorView.lineWrapping,
        mergeHighlightExtension(),
        sideConflictWidgetExtension(),
        mergeDecorationTheme(),
      ];
      if (langExt) exts.push(langExt);
      return exts;
    };

    theirsView = new EditorView({
      state: EditorState.create({ doc: theirs, extensions: sideExts(theirsLineNumComp) }),
      parent: theirsEl,
    });

    const resultExts = [
      theme,
      resultLineNumComp.of(lineNumExt),
      EditorView.lineWrapping,
      history(),
      mergeHighlightExtension(),
      conflictLineWidgetExtension(),
      mergeDecorationTheme(),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) return;
        const last = centerChunkPositions.length - 1;
        centerChunkPositions = centerChunkPositions.map((pos, i) =>
          update.changes.mapPos(pos, i === last ? 1 : -1),
        );
        scheduleRefresh();
      }),
    ];
    if (langExt) resultExts.push(langExt);

    resultView = new EditorView({
      state: EditorState.create({ doc: mergedContent, extensions: resultExts }),
      parent: resultEl,
    });

    oursView = new EditorView({
      state: EditorState.create({ doc: ours, extensions: sideExts(oursLineNumComp) }),
      parent: oursEl,
    });

    // Chunk anchors for scroll sync and connectors
    const centerDoc = resultView.state.doc;
    centerChunkPositions = centerChunkStartLines(newChunks).map((line0) =>
      line0 >= centerDoc.lines ? centerDoc.length : centerDoc.line(line0 + 1).from,
    );
    theirsChunkLines = sideChunkStartLines(newChunks, 'theirs');
    oursChunkLines = sideChunkStartLines(newChunks, 'ours');

    const callbacks = setConflictCallbacks.of({ decide, activate });
    theirsView.dispatch({ effects: callbacks });
    resultView.dispatch({ effects: callbacks });
    oursView.dispatch({ effects: callbacks });
    refreshDerived();

    for (const view of [theirsView, resultView, oursView]) {
      view.scrollDOM.addEventListener('scroll', () => onPanelScroll(view), { passive: true });
      const lead = () => { leader = view; };
      view.scrollDOM.addEventListener('wheel', lead, { passive: true });
      view.scrollDOM.addEventListener('pointerdown', lead);
      view.scrollDOM.addEventListener('touchstart', lead, { passive: true });
      view.contentDOM.addEventListener('keydown', lead);
    }
    leader = resultView;

    requestAnimationFrame(() => updateConnectors());
  }

  // ---------------------------------------------------------------------------
  // Navigation
  // ---------------------------------------------------------------------------

  function handleNextConflict() {
    scrollToConflict('forward');
  }

  function handlePrevConflict() {
    scrollToConflict('backward');
  }

  /**
   * Find the next/previous unresolved conflict, center it in the result
   * panel and let scroll sync bring the side panels along.
   */
  function scrollToConflict(direction: 'forward' | 'backward') {
    if (!resultView) return;
    const doc = resultView.state.doc;
    const hits = findPlaceholders(doc);
    if (hits.length === 0) return;

    const cursorLine = doc.lineAt(resultView.state.selection.main.head).number;
    const referenceLine = activeConflictIndex !== null
      ? hits.find((h) => h.marker.index === activeConflictIndex)?.lineNumber ?? cursorLine
      : cursorLine;

    let target;
    if (direction === 'forward') {
      target = hits.find((h) => h.lineNumber > referenceLine) ?? hits[0];
    } else {
      target = [...hits].reverse().find((h) => h.lineNumber < referenceLine) ?? hits[hits.length - 1];
    }

    const line = doc.line(target.lineNumber);
    activeConflictIndex = target.marker.index;
    leader = resultView;
    resultView.dispatch({
      selection: { anchor: line.from },
      effects: EditorView.scrollIntoView(line.from, { y: 'center' }),
    });
    refreshDerived();
    // The scroll effect applies on the next measure; align the sides after it
    // even when the result did not need to move.
    scheduleResync();
  }

  // ---------------------------------------------------------------------------
  // Resolve handling
  // ---------------------------------------------------------------------------

  /** Returns true if the content contains any git conflict markers. */
  function hasConflictMarkers(content: string): boolean {
    return /^<{7}[\s]|^={7}\s*$|^>{7}[\s]/m.test(content.replace(/\r\n/g, '\n'));
  }

  /** Handle resolve button click — show confirmation if conflict markers remain. */
  function handleResolveClick() {
    if (!resultView) return;
    const content = resultView.state.doc.toString();
    if (hasConflictMarkers(content) || hasPlaceholders(content)) {
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

  $effect(() => {
    // Read reactive deps so the effect re-runs on change.
    const _ours = ours;
    const _theirs = theirs;
    const _base = base;
    const _file = filename;
    // Tracked so a later real rebuild picks up the current mode, but a flip
    // alone does not rebuild (see the guard below). Live theme updates in an
    // open merge are sacrificed to preserve the in-progress resolution.
    void isDark;
    const _thEl = theirsEl;
    const _rEl = resultEl;
    const _oEl = oursEl;

    if (!_thEl || !_rEl || !_oEl) return;

    if (
      theirsView &&
      _ours === mountedOurs &&
      _theirs === mountedTheirs &&
      _base === mountedBase &&
      _file === mountedFile
    ) {
      return;
    }

    mountedOurs = _ours;
    mountedTheirs = _theirs;
    mountedBase = _base;
    mountedFile = _file;
    untrack(() => initEditors());
  });

  onDestroy(() => {
    theirsView?.destroy();
    resultView?.destroy();
    oursView?.destroy();
    theirsView = undefined;
    resultView = undefined;
    oursView = undefined;
  });
</script>

<svelte:window onresize={() => { scheduleConnectors(); scheduleSticky(); }} />

{#snippet stickyBar(side: ConflictSide, conflict: SideConflict)}
  <div
    class="sticky-conflict"
    class:active={conflict.active}
    class:decided={conflict.decision !== 'pending'}
    role="toolbar"
    aria-label={m.merge_conflict_label({ n: String(conflict.index + 1) })}
    onmousedown={() => { leader = side === 'theirs' ? theirsView : oursView; activate(conflict.index); }}
  >
    <span class="sticky-label">{"◆"} {m.merge_conflict_label({ n: String(conflict.index + 1) })}</span>
    <span class="sticky-count">{m.merge_side_conflict_count({ count: String(conflict.lineCount) })}</span>
    <span class="sticky-spacer"></span>
    {#if conflict.decision === 'pending'}
      <button
        type="button"
        class="sticky-accept"
        title={side === 'theirs' ? m.merge_accept_theirs() : m.merge_accept_ours()}
        onmousedown={(e) => { e.preventDefault(); decide(conflict.index, side, 'accepted'); }}
      >{side === 'theirs' ? '❯' : '❮'} {m.merge_accept()}</button>
      <button
        type="button"
        class="sticky-discard"
        title={side === 'theirs' ? m.merge_discard_theirs() : m.merge_discard_ours()}
        onmousedown={(e) => { e.preventDefault(); decide(conflict.index, side, 'discarded'); }}
      >{"✕"} {m.merge_discard()}</button>
    {:else}
      <span class="sticky-state" class:discarded={conflict.decision === 'discarded'}>
        {conflict.decision === 'accepted' ? m.merge_side_accepted() : m.merge_side_discarded()}
      </span>
    {/if}
  </div>
{/snippet}

<div class="merge-editor-wrapper">
  <div class="merge-toolbar">
    <span class="merge-filename">{filename}</span>
    <div class="merge-actions">
      <IconButton
        tone="default"
        icon={""}
        description={m.merge_prev_conflict()}
        onclick={handlePrevConflict}
      />
      <IconButton
        tone="default"
        icon={""}
        description={m.merge_next_conflict()}
        onclick={handleNextConflict}
      />
      <IconButton
        tone="default"
        icon={""}
        description={m.merge_undo()}
        onclick={handleUndo}
      />
      <IconButton
        tone="default"
        icon={""}
        description={m.merge_toggle_line_numbers()}
        active={showLineNumbers}
        onclick={toggleLineNumbers}
      />
      <IconButton
        tone="default"
        icon={""}
        description={m.merge_scroll_lock()}
        active={scrollLinked}
        onclick={toggleScrollLink}
      />
      <span class="conflict-counter" class:done={allResolved && totalConflicts > 0}>
        {m.merge_conflicts_counter({ resolved: String(resolvedCount), total: String(totalConflicts) })}
      </span>
      <Button
        variant="success"
        disabled={!allResolved && totalConflicts > 0}
        onclick={handleResolveClick}
      >
        {m.merge_mark_resolved()}
      </Button>
      {#if onCancel}
        <Button variant="danger" onclick={onCancel}>{m.merge_cancel()}</Button>
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
      <div class="panel-editor">
        <div class="panel-editor-host" bind:this={theirsEl}></div>
        {#if stickyTheirs}
          {@render stickyBar('theirs', stickyTheirs)}
        {/if}
      </div>
      <div class="connector-gap" style="width: {gapWidth}px">
        <svg bind:this={leftSvg} class="connector-svg"></svg>
      </div>
      <div class="panel-editor panel-editor-center">
        <div class="panel-editor-host" bind:this={resultEl}></div>
      </div>
      <div class="connector-gap" style="width: {gapWidth}px">
        <svg bind:this={rightSvg} class="connector-svg"></svg>
      </div>
      <div class="panel-editor">
        <div class="panel-editor-host" bind:this={oursEl}></div>
        {#if stickyOurs}
          {@render stickyBar('ours', stickyOurs)}
        {/if}
      </div>
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
    font-size: var(--font-size-sm);
    gap: 8px;
    flex-shrink: 0;
  }

  .merge-filename {
    font-family: var(--font-mono);
    color: var(--accent-primary);
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

  .conflict-counter {
    color: var(--accent-orange);
    font-size: var(--font-size-xs);
    font-weight: 600;
    padding: 0 4px;
  }

  .conflict-counter.done {
    color: var(--accent-green);
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
    font-size: var(--font-size-xs);
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
    color: var(--accent-primary);
  }

  .panel-editors {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .panel-editor {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    position: relative;
    /* Create stacking context so editor gutters don't escape into connector gaps */
    isolation: isolate;
  }

  .panel-editor-host {
    height: 100%;
  }

  .sticky-conflict {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 24px;
    padding: 2px 6px 2px 8px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-primary);
    background: color-mix(in srgb, var(--accent-purple) 22%, var(--bg-primary));
    border-left: 2px solid var(--accent-purple);
    border-bottom: 1px solid color-mix(in srgb, var(--accent-purple) 45%, transparent);
    box-shadow: 0 2px 6px color-mix(in srgb, var(--bg-primary) 60%, transparent);
  }

  .sticky-conflict.active {
    background: color-mix(in srgb, var(--accent-purple) 34%, var(--bg-primary));
  }

  .sticky-conflict.decided {
    background: color-mix(in srgb, var(--accent-green) 16%, var(--bg-primary));
    border-left-color: var(--accent-green);
    border-bottom-color: color-mix(in srgb, var(--accent-green) 35%, transparent);
  }

  .sticky-label {
    font-weight: 600;
    white-space: nowrap;
  }

  .sticky-count {
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .sticky-spacer {
    flex: 1;
  }

  .sticky-accept,
  .sticky-discard {
    border: 1px solid transparent;
    border-radius: 3px;
    padding: 2px 8px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    line-height: 16px;
    white-space: nowrap;
    cursor: pointer;
  }

  .sticky-accept {
    background: color-mix(in srgb, var(--accent-green) 25%, transparent);
    color: var(--accent-green);
  }

  .sticky-accept:hover {
    background: color-mix(in srgb, var(--accent-green) 40%, transparent);
    color: var(--text-primary);
  }

  .sticky-discard {
    background: none;
    color: var(--accent-red);
    opacity: 0.8;
  }

  .sticky-discard:hover {
    opacity: 1;
    border-color: color-mix(in srgb, var(--accent-red) 40%, transparent);
    background: color-mix(in srgb, var(--accent-red) 12%, transparent);
  }

  .sticky-state {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    padding: 1px 6px;
    border-radius: 3px;
    color: var(--accent-green);
    background: color-mix(in srgb, var(--accent-green) 18%, transparent);
  }

  .sticky-state.discarded {
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--text-secondary) 15%, transparent);
    text-decoration: line-through;
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
    overflow: hidden;
  }

  .connector-svg {
    position: absolute;
    inset: 0;
    pointer-events: none;
    overflow: hidden;
  }

  .panel-editor :global(.cm-editor) {
    height: 100%;
  }

  .panel-editor :global(.cm-scroller) {
    overflow: auto;
    font-family: 'Fira Code', var(--font-mono), monospace;
    font-size: var(--font-size-sm);
    line-height: 1.5;
  }
</style>
