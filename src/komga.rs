use crate::state::AppState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KomgaBook {
    pub id: String,
    pub name: String,
    pub series_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KomgaBookMetadata {
    pub title: String,
    #[serde(default)]
    pub authors: Vec<KomgaAuthor>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KomgaAuthor {
    pub name: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct BookSummary {
    pub id: String,
    pub title: String,
    pub author: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KomgaSeries {
    pub id: String,
    pub name: String,
    pub library_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KomgaPage<T> {
    pub content: Vec<T>,
    pub total_elements: Option<u64>,
}

/// Fetch libraries from Komga.
pub async fn get_libraries(state: &AppState) -> Result<Vec<Library>, String> {
    let komga = state.komga()?;
    let resp = state
        .http
        .get(format!("{}/api/v1/libraries", komga.url))
        .basic_auth(&komga.user, Some(&komga.password))
        .send()
        .await
        .map_err(|e| format!("Komga request failed: {e}"))?;

    resp.json::<Vec<Library>>()
        .await
        .map_err(|e| format!("Failed to parse libraries: {e}"))
}

/// Fetch series for a library from Komga.
pub async fn get_series(state: &AppState, library_id: &str) -> Result<Vec<KomgaSeries>, String> {
    let komga = state.komga()?;
    let resp = state
        .http
        .get(format!(
            "{}/api/v1/series?library_id={}&size=500&unpaged=true",
            komga.url, library_id
        ))
        .basic_auth(&komga.user, Some(&komga.password))
        .send()
        .await
        .map_err(|e| format!("Komga request failed: {e}"))?;

    let page = resp
        .json::<KomgaPage<KomgaSeries>>()
        .await
        .map_err(|e| format!("Failed to parse series: {e}"))?;

    Ok(page.content)
}

/// Fetch books for a series from Komga.
pub async fn get_books(state: &AppState, series_id: &str) -> Result<Vec<BookSummary>, String> {
    let komga = state.komga()?;
    let resp = state
        .http
        .get(format!(
            "{}/api/v1/series/{}/books?unpaged=true",
            komga.url, series_id
        ))
        .basic_auth(&komga.user, Some(&komga.password))
        .send()
        .await
        .map_err(|e| format!("Komga request failed: {e}"))?;

    let page = resp
        .json::<KomgaPage<KomgaBook>>()
        .await
        .map_err(|e| format!("Failed to parse books: {e}"))?;

    let mut summaries = Vec::new();
    for book in page.content {
        // Fetch metadata for each book
        let meta = get_book_metadata(state, &book.id).await.ok();
        summaries.push(BookSummary {
            id: book.id,
            title: meta
                .as_ref()
                .map(|m| m.title.clone())
                .unwrap_or(book.name),
            author: meta
                .and_then(|m| {
                    m.authors
                        .iter()
                        .find(|a| a.role == "writer")
                        .or(m.authors.first())
                        .map(|a| a.name.clone())
                })
                .unwrap_or_default(),
        });
    }

    Ok(summaries)
}

async fn get_book_metadata(
    state: &AppState,
    book_id: &str,
) -> Result<KomgaBookMetadata, String> {
    let komga = state.komga()?;
    let resp = state
        .http
        .get(format!("{}/api/v1/books/{}/metadata", komga.url, book_id))
        .basic_auth(&komga.user, Some(&komga.password))
        .send()
        .await
        .map_err(|e| format!("Komga request failed: {e}"))?;

    resp.json::<KomgaBookMetadata>()
        .await
        .map_err(|e| format!("Failed to parse metadata: {e}"))
}

/// Download EPUB file bytes from Komga.
pub async fn download_epub(state: &AppState, book_id: &str) -> Result<Vec<u8>, String> {
    let komga = state.komga()?;
    let resp = state
        .http
        .get(format!("{}/api/v1/books/{}/file", komga.url, book_id))
        .basic_auth(&komga.user, Some(&komga.password))
        .send()
        .await
        .map_err(|e| format!("Komga request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Komga returned status {}", resp.status()));
    }

    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to download EPUB: {e}"))
}


/// Download book thumbnail from Komga.
pub async fn download_thumbnail(state: &AppState, book_id: &str) -> Result<(Vec<u8>, String), String> {
    let komga = state.komga()?;
    let resp = state
        .http
        .get(format!(
            "{}/api/v1/books/{}/thumbnail",
            komga.url, book_id
        ))
        .basic_auth(&komga.user, Some(&komga.password))
        .send()
        .await
        .map_err(|e| format!("Komga request failed: {e}"))?;

    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to download thumbnail: {e}"))?
        .to_vec();

    Ok((bytes, content_type))
}
