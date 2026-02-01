# XML/HTML to SQLite Query System (WASM)

A WebAssembly-powered application that parses XML and HTML documents into SQLite databases and enables querying using CSS selectors or SQL.

## Features

- ✅ **Parse XML/HTML**: Load documents directly into an in-memory SQLite database
- ✅ **CSS Selector Queries**: Use familiar CSS selector syntax (`.class`, `#id`, `tag`, `[attr]`, combinators)
- ✅ **SQL Queries**: Execute raw SQL for complex queries
- ✅ **Results Export**: Export query results as JSON or CSV
- ✅ **Browser-Based**: Runs entirely in the browser using WebAssembly
- ✅ **Fast Performance**: Leveraging Rust's performance and SQLite's efficiency

## Architecture

```
┌─────────────────┐
│  React Frontend │
│   (TypeScript)  │
└────────┬────────┘
         │
    ┌────▼────┐
    │  WASM   │  ◄─── Rust (xmlsql-wasm)
    │ Module  │       - XML/HTML Parser
    └────┬────┘       - CSS → SQL Transpiler
         │            - SQLite Database
    ┌────▼────┐
    │ SQLite  │
    │ In-Mem  │
    └─────────┘
```

## Database Schema

The application uses the following SQLite schema:

```sql
CREATE TABLE documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    root_node_id INTEGER,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE nodes (
    id INTEGER PRIMARY KEY,
    document_id INTEGER NOT NULL,
    parent_id INTEGER,
    tag_name TEXT NOT NULL,
    text_content TEXT,
    depth INTEGER NOT NULL,
    position INTEGER NOT NULL
);

CREATE TABLE attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    node_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    value TEXT
);
```

## Prerequisites

### Rust Development
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Node.js Development
- Node.js 18+ (Download from https://nodejs.org/)

## Build Instructions

### 1. Build WASM Module

```bash
# From project root
wasm-pack build --target web --out-dir web/public/wasm
```

This compiles the Rust code to WebAssembly and generates JavaScript bindings.

### 2. Install Frontend Dependencies

```bash
cd web
npm install
```

### 3. Run Development Server

```bash
npm run dev
```

The application will be available at `http://localhost:5173`

### 4. Build for Production

```bash
npm run build
```

## Usage

### 1. Load a Document

- Click the drop zone or drag & drop an XML or HTML file
- Example files are provided in `/examples/`

### 2. Query with CSS Selectors

```css
/* Select all div elements */
div

/* Select elements by class */
.post-title

/* Select by ID */
#post-1

/* Attribute selectors */
[data-category="technology"]
[data-minutes]

/* Combinators */
article > header    /* Direct child */
main article        /* Descendant */

/* Complex queries */
article.post.featured > header > h1.post-title
```

### 3. Query with SQL

```sql
-- Get all nodes
SELECT * FROM nodes;

-- Find nodes by tag name
SELECT * FROM nodes WHERE tag_name = 'article';

-- Count nodes by tag
SELECT tag_name, COUNT(*) as count 
FROM nodes 
GROUP BY tag_name 
ORDER BY count DESC;

-- Find nodes with specific attributes
SELECT n.*, a.name, a.value
FROM nodes n
JOIN attributes a ON a.node_id = n.id
WHERE a.name = 'class' AND a.value LIKE '%post%';

-- Complex joins
SELECT DISTINCT n1.tag_name as parent_tag, n2.tag_name as child_tag
FROM nodes n1
JOIN nodes n2 ON n2.parent_id = n1.id
ORDER BY parent_tag, child_tag;
```

### 4. Export Results

- **JSON**: Full result set with column names and data
- **CSV**: Spreadsheet-compatible format

## CSS Selector Support

| Selector Type | Example | SQL Translation |
|---------------|---------|-----------------|
| Tag | `div` | `tag_name = 'div'` |
| Class | `.container` | `JOIN attributes WHERE name='class' AND value LIKE '%container%'` |
| ID | `#main` | `JOIN attributes WHERE name='id' AND value='main'` |
| Attribute Exists | `[href]` | `JOIN attributes WHERE name='href'` |
| Attribute Equals | `[type="text"]` | `JOIN attributes WHERE name='type' AND value='text'` |
| Attribute Contains | `[class*="post"]` | `WHERE value LIKE '%post%'` |
| Attribute Starts With | `[href^="https"]` | `WHERE value LIKE 'https%'` |
| Attribute Ends With | `[src$=".png"]` | `WHERE value LIKE '%.png'` |
| Child Combinator | `div > p` | `JOIN nodes ON parent_id = ...` |
| Descendant Combinator | `article p` | `WITH RECURSIVE descendants...` |

## Example Queries

### XML Example (books.xml)

```css
/* All books */
book

/* Fiction books */
book[category="fiction"]

/* Books with title containing specific text */
book > title

/* Get all authors */
author
```

```sql
-- Books published after 1950
SELECT * FROM nodes 
WHERE tag_name = 'book' 
AND id IN (
    SELECT parent_id FROM nodes 
    WHERE tag_name = 'year' 
    AND CAST(text_content AS INTEGER) > 1950
);

-- Count books by category
SELECT a.value as category, COUNT(*) as count
FROM nodes n
JOIN attributes a ON a.node_id = n.id
WHERE n.tag_name = 'book' AND a.name = 'category'
GROUP BY a.value;
```

### HTML Example (blog.html)

```css
/* All article titles */
article > header > h1.post-title

/* Featured posts */
article.featured

/* Posts in technology category */
article[data-category="technology"]

/* All tags */
.tag

/* Recent posts sidebar */
#recent-posts a
```

```sql
-- Get all post metadata
SELECT 
    n.id,
    (SELECT text_content FROM nodes WHERE parent_id = n.id AND tag_name = 'h1' LIMIT 1) as title,
    a.value as category
FROM nodes n
JOIN attributes a ON a.node_id = n.id
WHERE n.tag_name = 'article' 
AND a.name = 'data-category';

-- Count posts by author
SELECT 
    (SELECT text_content FROM nodes 
     WHERE parent_id = n.id 
     AND tag_name = 'span' 
     AND id IN (SELECT node_id FROM attributes WHERE name='class' AND value='author')
     LIMIT 1) as author,
    COUNT(*) as post_count
FROM nodes n
WHERE n.tag_name = 'article'
GROUP BY author;
```

## Development

### Project Structure

```
xmlsql-wasm/
├── Cargo.toml                 # Rust dependencies
├── src/
│   ├── lib.rs                 # WASM entry point
│   ├── parser.rs              # XML/HTML parsing
│   ├── selector.rs            # CSS → SQL transpiler
│   └── database.rs            # SQLite schema
├── web/
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.tsx            # Main React component
│   │   ├── components/        # React components
│   │   └── types/             # TypeScript types
│   └── public/
│       └── wasm/              # Compiled WASM files
└── examples/                  # Sample XML/HTML files
```

### Running Tests

```bash
# Rust tests
cargo test

# Frontend tests (if added)
cd web
npm test
```

### Performance Optimization

The WASM binary is optimized for size:

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
```

Further optimize with:

```bash
wasm-pack build --target web --release
wasm-opt -Oz -o optimized.wasm pkg/xmlsql_wasm_bg.wasm
```

## Limitations

### Current Limitations
- Sibling combinators (`+`, `~`) not yet implemented
- Pseudo-classes (`:first-child`, `:nth-child`) not supported
- Database export to file not implemented
- No persistent storage (in-memory only)

### Planned Features
- [ ] Full CSS selector spec support
- [ ] XPath query support
- [ ] Database persistence (IndexedDB)
- [ ] Import existing SQLite databases
- [ ] Query history and saved queries
- [ ] Visual query builder
- [ ] Document diff/compare tool

## Browser Compatibility

Requires a modern browser with WebAssembly support:
- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

## Performance

Typical performance on modern hardware:
- Parsing: ~10,000 nodes/second
- Simple queries: <10ms
- Complex queries: 10-100ms
- WASM bundle size: ~200KB (gzipped)

## Contributing

Contributions are welcome! Areas for improvement:
- Additional CSS selector features
- Performance optimizations
- UI/UX enhancements
- Documentation
- Test coverage

## License

MIT License - see LICENSE file for details

## Acknowledgments

Based on concepts from:
- [xmlsql by necessary-nu](https://github.com/necessary-nu/xmlsql)
- [sqlite-wasm-rs](https://github.com/Spxg/sqlite-wasm-rs)
- [rusqlite](https://github.com/rusqlite/rusqlite)

## Troubleshooting

### WASM module fails to load
- Ensure you've built the WASM module: `wasm-pack build --target web`
- Check browser console for errors
- Verify WASM file exists in `web/public/wasm/`

### Parser errors
- Check that XML/HTML is well-formed
- Look for unmatched tags or invalid syntax
- Try validating with an online XML/HTML validator

### Query errors
- Verify CSS selector syntax
- For SQL, check table/column names match schema
- Use browser DevTools to inspect generated SQL (console logs)

## Support

For issues and questions:
- Open an issue on GitHub
- Check existing documentation
- Review example files in `/examples/`
