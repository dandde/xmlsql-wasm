use crate::NodeData;
use std::collections::HashMap;

pub fn parse_xml_to_nodes(content: &str) -> Result<Vec<NodeData>, String> {
    let doc =
        roxmltree::Document::parse(content).map_err(|e| format!("XML parsing error: {}", e))?;

    let mut nodes = Vec::new();
    let mut node_id = 1i64;

    fn traverse_xml(
        node: roxmltree::Node,
        parent_id: Option<i64>,
        depth: i32,
        nodes: &mut Vec<NodeData>,
        node_id: &mut i64,
    ) {
        if node.is_element() {
            let current_id = *node_id;
            *node_id += 1;

            let mut attributes = HashMap::new();
            for attr in node.attributes() {
                attributes.insert(attr.name().to_string(), attr.value().to_string());
            }

            // Collect text content correctly (including mixed content)
            let mut text_parts = Vec::new();
            for child in node.children() {
                if child.is_text() {
                    if let Some(text) = child.text() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            text_parts.push(trimmed.to_string());
                        }
                    }
                }
            }

            let text_content = if text_parts.is_empty() {
                None
            } else {
                Some(text_parts.join(" "))
            };

            nodes.push(NodeData {
                id: current_id,
                tag_name: node.tag_name().name().to_string(),
                text_content,
                attributes,
                parent_id,
                depth,
            });

            for child in node.children() {
                if child.is_element() {
                    traverse_xml(child, Some(current_id), depth + 1, nodes, node_id);
                }
            }
        }
    }

    // roxmltree::Document::root_element() returns Node (not Option)
    traverse_xml(doc.root_element(), None, 0, &mut nodes, &mut node_id);

    Ok(nodes)
}

pub fn parse_html_to_nodes(content: &str) -> Result<Vec<NodeData>, String> {
    use ego_tree::NodeRef;
    use scraper::{Html, Node as ScraperNode};

    let document = Html::parse_document(content);
    let mut nodes = Vec::new();
    let mut node_id = 1i64;

    fn traverse_html(
        node: NodeRef<ScraperNode>,
        parent_id: Option<i64>,
        depth: i32,
        nodes: &mut Vec<NodeData>,
        node_id: &mut i64,
    ) {
        match node.value() {
            ScraperNode::Element(element) => {
                let current_id = *node_id;
                *node_id += 1;

                let mut attributes = HashMap::new();
                for (name, value) in element.attrs() {
                    attributes.insert(name.to_string(), value.to_string());
                }

                // Get text content from direct children
                let text_content = node
                    .children()
                    .filter_map(|child| match child.value() {
                        ScraperNode::Text(text) => {
                            let t = text.trim();
                            if t.is_empty() {
                                None
                            } else {
                                Some(t.to_string())
                            }
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let text_content = if text_content.is_empty() {
                    None
                } else {
                    Some(text_content)
                };

                nodes.push(NodeData {
                    id: current_id,
                    tag_name: element.name().to_string(),
                    text_content,
                    attributes,
                    parent_id,
                    depth,
                });

                for child in node.children() {
                    traverse_html(child, Some(current_id), depth + 1, nodes, node_id);
                }
            }
            _ => {
                // Skip non-element nodes (text is handled by parent, comments skipped)
            }
        }
    }

    // Parse root element (<html>)
    // document.root_element() returns ElementRef, which derefs to NodeRef
    traverse_html(*document.root_element(), None, 0, &mut nodes, &mut node_id);

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"
            <?xml version="1.0"?>
            <root>
                <child id="1">Content</child>
                <child id="2">More content</child>
            </root>
        "#;

        let nodes = parse_xml_to_nodes(xml.trim()).unwrap();
        assert!(nodes.len() >= 3); // root + 2 children

        let root = &nodes[0];
        assert_eq!(root.tag_name, "root");
        assert_eq!(root.parent_id, None);
    }

    #[test]
    fn test_parse_simple_html() {
        let html = r#"
            <html>
                <body>
                    <div class="container">
                        <p id="intro">Hello World</p>
                    </div>
                </body>
            </html>
        "#;

        let nodes = parse_html_to_nodes(html).unwrap();
        assert!(!nodes.is_empty());

        // Find the div with class="container"
        let container = nodes.iter().find(|n| {
            n.tag_name == "div" && n.attributes.get("class") == Some(&"container".to_string())
        });
        assert!(container.is_some());
    }

    #[test]
    fn test_html_root_element_exists() {
        let html = "<html><body></body></html>";
        let nodes = parse_html_to_nodes(html).unwrap();

        // Should find "html" tag
        let html_node = nodes.iter().find(|n| n.tag_name == "html");
        assert!(html_node.is_some(), "HTML root element missing");
        if let Some(node) = html_node {
            assert_eq!(node.parent_id, None);
        }
    }

    #[test]
    fn test_xml_mixed_content() {
        // roxmltree text() behavior check
        let xml = "<root>A<b>Bold</b>C</root>";
        let nodes = parse_xml_to_nodes(xml).unwrap();
        let root = nodes.iter().find(|n| n.tag_name == "root").unwrap();

        println!("Root text content: {:?}", root.text_content);

        assert!(root.text_content.is_some(), "Mixed content text lost");
        let text = root.text_content.as_ref().unwrap();
        assert!(text.contains("A"), "Missing 'A'");
        assert!(text.contains("C"), "Missing 'C'");
    }
}
