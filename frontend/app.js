// --- State ---
let currentBook = null;
let uiVisible = false;
let settingsVisible = false;
let colorPickerVisible = false;
let bookmarks = []; // [{ chunk_index, color, label }]

// Modes: 'zen' (~5 sentences), 'flow' (~10 sentences)
let readingMode = 'zen';
let flowView = 'full'; // 'full' | 'top' | 'bottom'

// Settings (persisted to localStorage)
let settings = loadSettings();

// --- DOM refs ---
const libraryView = document.getElementById('library-view');
const booksView = document.getElementById('books-view');
const readingView = document.getElementById('reading-view');
const librarySelect = document.getElementById('library-select');
const seriesList = document.getElementById('series-list');
const booksList = document.getElementById('books-list');
const seriesTitle = document.getElementById('series-title');
const chunkText = document.getElementById('chunk-text');
const chapterName = document.getElementById('chapter-name');
const readingHeader = document.getElementById('reading-header');
const readingFooter = document.getElementById('reading-footer');
const progressFill = document.getElementById('progress-fill');
const progressText = document.getElementById('progress-text');
const fileUpload = document.getElementById('file-upload');
const modeIndicator = document.getElementById('mode-indicator');
const settingsPanel = document.getElementById('settings-panel');
const fontSelect = document.getElementById('font-select');
const customFontInput = document.getElementById('custom-font');
const sizeValue = document.getElementById('size-value');
const lhValue = document.getElementById('lh-value');
const bookmarkBtn = document.getElementById('bookmark-btn');
const edgeStrip = document.getElementById('edge-strip');
const edgePosition = document.getElementById('edge-position');
const edgeChapters = document.getElementById('edge-chapters');
const edgeBookmarks = document.getElementById('edge-bookmarks');
const colorPicker = document.getElementById('color-picker');

// --- Settings persistence ---
function loadSettings() {
    try {
        const saved = JSON.parse(localStorage.getItem('zen-settings'));
        return {
            theme: saved?.theme || 'dark',
            fontSize: saved?.fontSize || 18,
            lineHeight: saved?.lineHeight || 1.8,
            fontFamily: saved?.fontFamily || 'system',
            customFont: saved?.customFont || '',
        };
    } catch {
        return { theme: 'dark', fontSize: 18, lineHeight: 1.8, fontFamily: 'system', customFont: '' };
    }
}

function saveSettings() {
    localStorage.setItem('zen-settings', JSON.stringify(settings));
}

function applySettings() {
    // Theme
    document.body.setAttribute('data-theme', settings.theme);

    // Font size
    document.documentElement.style.setProperty('--font-size', settings.fontSize + 'px');
    sizeValue.textContent = settings.fontSize;

    // Line height
    document.documentElement.style.setProperty('--line-height', settings.lineHeight);
    lhValue.textContent = settings.lineHeight.toFixed(1);

    // Font family
    let fontStack;
    switch (settings.fontFamily) {
        case 'serif':
            fontStack = 'Georgia, "Noto Serif", "Noto Serif CJK SC", serif';
            break;
        case 'monaspice':
            fontStack = '"MonaspiceRn Nerd Font", "MonaspiceRn NFM", monospace';
            break;
        case 'custom':
            fontStack = settings.customFont
                ? `"${settings.customFont}", sans-serif`
                : 'sans-serif';
            break;
        default:
            fontStack = '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Noto Sans", "Noto Sans CJK SC", "Noto Sans CJK TC", sans-serif';
    }
    document.documentElement.style.setProperty('--font-family', fontStack);

    // Update swatch active state
    document.querySelectorAll('.swatch').forEach(s => {
        s.classList.toggle('active', s.dataset.theme === settings.theme);
    });

    // Update font select
    fontSelect.value = settings.fontFamily;
    customFontInput.classList.toggle('hidden', settings.fontFamily !== 'custom');
    customFontInput.value = settings.customFont;

    saveSettings();
}

// --- Sentence break markers ---
function highlightBreaks(text) {
    // Escape HTML first
    const escaped = text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');

    // Convert newlines to <br> (merged short chunks use \n as separator)
    // Highlight sentence-ending punctuation with marker class
    return escaped
        .replace(/\n/g, '<br>')
        .replace(/([.?!。？！])/g, '<span class="marker">$1</span>');
}

// --- View switching ---
function showView(view) {
    [libraryView, booksView, readingView].forEach(v => v.classList.remove('active'));
    view.classList.add('active');
    settingsVisible = false;
    settingsPanel.classList.add('hidden');
}

// --- Mode ---
function toggleMode() {
    readingMode = readingMode === 'zen' ? 'flow' : 'zen';
    flowView = 'full';
    updateModeIndicator();
    renderChunk();
}

function updateModeIndicator() {
    modeIndicator.textContent = readingMode === 'zen' ? 'Z' : 'F';
}

// --- Library ---
async function loadLibraries() {
    try {
        const libs = await fetch('/api/libraries').then(r => r.json());
        librarySelect.innerHTML = libs.map(l =>
            `<option value="${l.id}">${l.name}</option>`
        ).join('');
        if (libs.length > 0) {
            loadSeries(libs[0].id);
        }
    } catch (e) {
        seriesList.innerHTML = '<div class="loading">Komga not configured. Upload an EPUB to read.</div>';
    }
}

librarySelect.addEventListener('change', () => {
    loadSeries(librarySelect.value);
});

async function loadSeries(libraryId) {
    seriesList.innerHTML = '<div class="loading">Loading...</div>';
    try {
        const series = await fetch(`/api/series?libraryId=${libraryId}`).then(r => r.json());
        seriesList.innerHTML = series.map(s =>
            `<div class="series-item" data-id="${s.id}">${s.name}</div>`
        ).join('');

        seriesList.querySelectorAll('.series-item').forEach(el => {
            el.addEventListener('click', () => {
                showBooks(el.dataset.id, el.textContent);
            });
        });
    } catch (e) {
        seriesList.innerHTML = '<div class="loading">Failed to load series.</div>';
    }
}

// --- Books ---
async function showBooks(seriesId, title) {
    seriesTitle.textContent = title;
    showView(booksView);
    booksList.innerHTML = '<div class="loading">Loading...</div>';

    try {
        const books = await fetch(`/api/books?seriesId=${seriesId}`).then(r => r.json());
        booksList.innerHTML = books.map(b => `
            <div class="book-card" data-id="${b.id}">
                <img src="/api/books/${b.id}/thumbnail" alt="" loading="lazy">
                <div class="book-title">${b.title}</div>
                <div class="book-author">${b.author}</div>
            </div>
        `).join('');

        booksList.querySelectorAll('.book-card').forEach(el => {
            el.addEventListener('click', () => openBook(el.dataset.id));
        });
    } catch (e) {
        booksList.innerHTML = '<div class="loading">Failed to load books.</div>';
    }
}

document.getElementById('back-to-library').addEventListener('click', () => showView(libraryView));

// --- Reading ---
async function openBook(bookId) {
    showView(readingView);
    chunkText.textContent = 'Loading...';
    chapterName.textContent = '';
    uiVisible = false;
    readingHeader.classList.add('hidden');
    readingFooter.classList.add('hidden');
    updateModeIndicator();

    try {
        const [chunks, progress] = await Promise.all([
            fetch(`/api/books/${bookId}/chunks`).then(r => r.json()),
            fetch(`/api/books/${bookId}/progress`).then(r => r.json()),
        ]);

        bookmarks = progress.bookmarks || [];
        currentBook = {
            id: bookId,
            chunks,
            chunkIndex: progress.chunk_index || 0,
        };

        renderChunk();
    } catch (e) {
        chunkText.textContent = 'Failed to load book.';
    }
}

async function openUploadedBook(chunks) {
    showView(readingView);
    uiVisible = false;
    readingHeader.classList.add('hidden');
    readingFooter.classList.add('hidden');
    updateModeIndicator();
    bookmarks = [];

    currentBook = {
        id: 'upload_' + Date.now(),
        chunks,
        chunkIndex: 0,
    };

    renderChunk();
}

function getCurrentDisplay() {
    if (!currentBook) return { text: '', chapter: '' };
    const { chunks, chunkIndex } = currentBook;
    const chunk = chunks.chunks[chunkIndex];
    if (!chunk) return { text: 'End of book.', chapter: '' };

    if (readingMode === 'zen') {
        return { text: chunk.text, chapter: chunk.chapter };
    }

    // Flow mode: merge current + next chunk
    const next = chunks.chunks[chunkIndex + 1];
    const fullText = next ? chunk.text + ' ' + next.text : chunk.text;

    if (flowView === 'top') {
        return { text: chunk.text, chapter: chunk.chapter };
    } else if (flowView === 'bottom') {
        return { text: next ? next.text : chunk.text, chapter: chunk.chapter };
    }
    return { text: fullText, chapter: chunk.chapter };
}

function renderChunk() {
    if (!currentBook) return;

    const display = getCurrentDisplay();
    // Use innerHTML for sentence break highlighting
    chunkText.innerHTML = highlightBreaks(display.text);
    chapterName.textContent = display.chapter;

    // Update progress
    const { chunks, chunkIndex } = currentBook;
    const pct = chunks.total_chunks > 0
        ? ((chunkIndex + 1) / chunks.total_chunks * 100)
        : 0;
    progressFill.style.width = pct + '%';
    progressText.textContent = `${chunkIndex + 1} / ${chunks.total_chunks}`;

    updateEdgeStrip();
    updateBookmarkBtn();
    saveProgress();
}

// --- Edge strip (right-side overview) ---
function updateEdgeStrip() {
    if (!currentBook) return;
    const { chunks, chunkIndex } = currentBook;
    const total = chunks.total_chunks;
    if (total === 0) return;

    const stripHeight = edgeStrip.clientHeight;

    // Current position indicator
    const posPct = chunkIndex / total;
    edgePosition.style.top = (posPct * stripHeight) + 'px';

    // Chapter ticks
    edgeChapters.innerHTML = chunks.chapters.map(ch => {
        const top = (ch.start_chunk / total) * stripHeight;
        return `<div class="edge-chapter-tick" style="top:${top}px"></div>`;
    }).join('');

    // Bookmark ticks
    edgeBookmarks.innerHTML = bookmarks.map((bm, i) => {
        const top = (bm.chunk_index / total) * stripHeight;
        return `<div class="edge-bookmark-tick" data-idx="${i}" style="top:${top}px;background:${bm.color}"></div>`;
    }).join('');

    // Click bookmark ticks to jump
    edgeBookmarks.querySelectorAll('.edge-bookmark-tick').forEach(el => {
        el.addEventListener('click', (e) => {
            e.stopPropagation();
            const bm = bookmarks[parseInt(el.dataset.idx)];
            if (bm) {
                currentBook.chunkIndex = bm.chunk_index;
                renderChunk();
            }
        });
    });
}

// Click on edge strip to jump to position
edgeStrip.addEventListener('click', (e) => {
    if (!currentBook || e.target.classList.contains('edge-bookmark-tick')) return;
    const rect = edgeStrip.getBoundingClientRect();
    const pct = (e.clientY - rect.top) / rect.height;
    const idx = Math.round(pct * (currentBook.chunks.total_chunks - 1));
    currentBook.chunkIndex = Math.max(0, Math.min(idx, currentBook.chunks.total_chunks - 1));
    renderChunk();
});

// --- Bookmarks ---
function updateBookmarkBtn() {
    if (!currentBook) return;
    const isBookmarked = bookmarks.some(b => b.chunk_index === currentBook.chunkIndex);
    bookmarkBtn.classList.toggle('bookmarked', isBookmarked);
    bookmarkBtn.style.color = isBookmarked
        ? bookmarks.find(b => b.chunk_index === currentBook.chunkIndex).color
        : '';
}

function addBookmark(color) {
    if (!currentBook) return;
    // Remove existing bookmark at this position
    bookmarks = bookmarks.filter(b => b.chunk_index !== currentBook.chunkIndex);
    bookmarks.push({
        chunk_index: currentBook.chunkIndex,
        color: color,
        label: '',
    });
    bookmarks.sort((a, b) => a.chunk_index - b.chunk_index);
    saveBookmarks();
    updateEdgeStrip();
    updateBookmarkBtn();
}

function removeBookmark() {
    if (!currentBook) return;
    bookmarks = bookmarks.filter(b => b.chunk_index !== currentBook.chunkIndex);
    saveBookmarks();
    updateEdgeStrip();
    updateBookmarkBtn();
}

let bmSaveTimeout = null;
function saveBookmarks() {
    if (!currentBook) return;
    clearTimeout(bmSaveTimeout);
    bmSaveTimeout = setTimeout(() => {
        fetch(`/api/books/${currentBook.id}/bookmarks`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(bookmarks),
        }).catch(() => {});
    }, 300);
}

// Bookmark button: tap to open color picker
bookmarkBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    const isBookmarked = bookmarks.some(b => b.chunk_index === currentBook?.chunkIndex);
    if (isBookmarked) {
        // Already bookmarked — remove it
        removeBookmark();
        colorPickerVisible = false;
        colorPicker.classList.add('hidden');
    } else {
        // Show color picker
        colorPickerVisible = !colorPickerVisible;
        colorPicker.classList.toggle('hidden', !colorPickerVisible);
    }
});

// Color picker: pick a color to add bookmark
colorPicker.addEventListener('click', (e) => {
    const dot = e.target.closest('.color-dot');
    if (!dot) return;
    addBookmark(dot.dataset.color);
    colorPickerVisible = false;
    colorPicker.classList.add('hidden');
});

let saveTimeout = null;
function saveProgress() {
    if (!currentBook) return;
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
        fetch(`/api/books/${currentBook.id}/progress`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                chunk_index: currentBook.chunkIndex,
                total_chunks: currentBook.chunks.total_chunks,
            }),
        }).catch(() => {});
    }, 500);
}

function nextChunk() {
    if (!currentBook) return;
    flowView = 'full';
    const step = readingMode === 'flow' ? 2 : 1;
    const next = currentBook.chunkIndex + step;
    if (next < currentBook.chunks.total_chunks) {
        currentBook.chunkIndex = next;
        renderChunk();
    } else if (currentBook.chunkIndex < currentBook.chunks.total_chunks - 1) {
        currentBook.chunkIndex = currentBook.chunks.total_chunks - 1;
        renderChunk();
    }
}

function prevChunk() {
    if (!currentBook) return;
    flowView = 'full';
    const step = readingMode === 'flow' ? 2 : 1;
    const prev = currentBook.chunkIndex - step;
    if (prev >= 0) {
        currentBook.chunkIndex = prev;
        renderChunk();
    } else if (currentBook.chunkIndex > 0) {
        currentBook.chunkIndex = 0;
        renderChunk();
    }
}

function flowSplitUp() {
    if (readingMode !== 'flow') return;
    flowView = flowView === 'top' ? 'full' : 'top';
    renderChunk();
}

function flowSplitDown() {
    if (readingMode !== 'flow') return;
    flowView = flowView === 'bottom' ? 'full' : 'bottom';
    renderChunk();
}

function toggleUI() {
    uiVisible = !uiVisible;
    readingHeader.classList.toggle('hidden', !uiVisible);
    readingFooter.classList.toggle('hidden', !uiVisible);
    if (!uiVisible) {
        settingsVisible = false;
        settingsPanel.classList.add('hidden');
    }
}

// --- Settings panel ---
document.getElementById('settings-toggle').addEventListener('click', (e) => {
    e.stopPropagation();
    settingsVisible = !settingsVisible;
    settingsPanel.classList.toggle('hidden', !settingsVisible);
});

// Theme swatches
document.getElementById('theme-swatches').addEventListener('click', (e) => {
    const swatch = e.target.closest('.swatch');
    if (!swatch) return;
    settings.theme = swatch.dataset.theme;
    applySettings();
});

// Font size
document.getElementById('size-down').addEventListener('click', () => {
    settings.fontSize = Math.max(12, settings.fontSize - 1);
    applySettings();
    if (currentBook) renderChunk();
});
document.getElementById('size-up').addEventListener('click', () => {
    settings.fontSize = Math.min(32, settings.fontSize + 1);
    applySettings();
    if (currentBook) renderChunk();
});

// Line height
document.getElementById('lh-down').addEventListener('click', () => {
    settings.lineHeight = Math.max(1.0, +(settings.lineHeight - 0.2).toFixed(1));
    applySettings();
});
document.getElementById('lh-up').addEventListener('click', () => {
    settings.lineHeight = Math.min(3.0, +(settings.lineHeight + 0.2).toFixed(1));
    applySettings();
});

// Font family
fontSelect.addEventListener('change', () => {
    settings.fontFamily = fontSelect.value;
    applySettings();
});
customFontInput.addEventListener('input', () => {
    settings.customFont = customFontInput.value;
    applySettings();
});

// Mode indicator click
modeIndicator.addEventListener('click', (e) => {
    e.stopPropagation();
    toggleMode();
});

// --- Scroll wheel navigation ---
let scrollCooldown = false;
readingView.addEventListener('wheel', (e) => {
    if (!readingView.classList.contains('active')) return;
    if (scrollCooldown) return;
    e.preventDefault();

    if (e.deltaY > 0) nextChunk();
    else if (e.deltaY < 0) prevChunk();

    scrollCooldown = true;
    setTimeout(() => { scrollCooldown = false; }, 200);
}, { passive: false });

// --- Touch navigation ---
let touchStartX = 0;
let touchStartY = 0;

readingView.addEventListener('touchstart', (e) => {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
}, { passive: true });

readingView.addEventListener('touchend', (e) => {
    // Ignore if settings panel is open
    if (settingsVisible) return;

    const dx = e.changedTouches[0].clientX - touchStartX;
    const dy = e.changedTouches[0].clientY - touchStartY;

    // Vertical swipe in flow mode = split view
    if (readingMode === 'flow' && Math.abs(dy) > Math.abs(dx) && Math.abs(dy) > 50) {
        if (dy < 0) flowSplitDown();
        else flowSplitUp();
        return;
    }

    if (Math.abs(dy) > Math.abs(dx) && Math.abs(dy) > 30) return;

    if (Math.abs(dx) > 50) {
        if (dx < 0) nextChunk();
        else prevChunk();
    } else {
        const screenW = window.innerWidth;
        const x = e.changedTouches[0].clientX;
        const centerZone = screenW * 0.3;
        const centerStart = (screenW - centerZone) / 2;

        if (x > centerStart && x < centerStart + centerZone) {
            toggleUI();
        } else if (x > screenW / 2) {
            nextChunk();
        } else {
            prevChunk();
        }
    }
});

// --- Keyboard navigation ---
document.addEventListener('keydown', (e) => {
    if (!readingView.classList.contains('active')) return;
    // Don't capture when typing in settings inputs
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;

    switch (e.key) {
        case 'j':
        case 'ArrowRight':
        case ' ':
            e.preventDefault();
            nextChunk();
            break;
        case 'k':
        case 'ArrowLeft':
            e.preventDefault();
            prevChunk();
            break;
        case 'ArrowUp':
            e.preventDefault();
            flowSplitUp();
            break;
        case 'ArrowDown':
            e.preventDefault();
            flowSplitDown();
            break;
        case 'm':
            toggleMode();
            break;
        case 'b':
            // Quick bookmark with blue
            bookmarkBtn.click();
            break;
        case 'q':
        case 'Escape':
            if (colorPickerVisible) {
                colorPickerVisible = false;
                colorPicker.classList.add('hidden');
            } else if (settingsVisible) {
                settingsVisible = false;
                settingsPanel.classList.add('hidden');
            } else {
                showView(libraryView);
            }
            break;
    }
});

// Close settings/color picker when clicking outside
document.addEventListener('click', (e) => {
    if (settingsVisible && !settingsPanel.contains(e.target) && e.target.id !== 'settings-toggle') {
        settingsVisible = false;
        settingsPanel.classList.add('hidden');
    }
    if (colorPickerVisible && !colorPicker.contains(e.target) && e.target.id !== 'bookmark-btn') {
        colorPickerVisible = false;
        colorPicker.classList.add('hidden');
    }
});

// --- Back button ---
document.getElementById('back-to-books').addEventListener('click', () => {
    showView(libraryView);
});

// --- File upload ---
fileUpload.addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    const formData = new FormData();
    formData.append('file', file);

    chunkText.textContent = 'Parsing...';
    showView(readingView);

    try {
        const resp = await fetch('/api/upload', { method: 'POST', body: formData });
        if (!resp.ok) throw new Error(await resp.text());
        const chunks = await resp.json();
        openUploadedBook(chunks);
    } catch (err) {
        chunkText.textContent = 'Failed to parse EPUB: ' + err.message;
    }

    fileUpload.value = '';
});

// --- Init ---
applySettings();
updateModeIndicator();
loadLibraries();
