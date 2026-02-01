use std::fmt;

#[derive(Debug)]
pub enum SelectorError {
    ParseError(String),
    UnsupportedFeature(String),
}

impl fmt::Display for SelectorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SelectorError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SelectorError::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    TagName(String),
    Class(String),
    Id(String),
    Attribute {
        name: String,
        value: Option<String>,
        operator: AttributeOperator,
    },
    Combinator(Combinator),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Combinator {
    Descendant,     // " "
    Child,          // ">"
    NextSibling,    // "+"
    GeneralSibling, // "~"
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeOperator {
    Exists,     // [attr]
    Equals,     // [attr=value]
    Contains,   // [attr*=value]
    StartsWith, // [attr^=value]
    EndsWith,   // [attr$=value]
    WordMatch,  // [attr~=value]
}

pub fn css_to_sql(selector: &str) -> Result<String, String> {
    let tokens = tokenize(selector)?;
    generate_sql(&tokens)
}

fn tokenize(selector: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = selector.trim().chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' => {
                chars.next();
                // Check if this is a descendant combinator
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_whitespace() {
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(&next) = chars.peek() {
                    if next != '>' && next != '+' && next != '~' {
                        // Don't emit descendant if previous token was a combinator
                        let last_is_combinator = tokens
                            .last()
                            .map_or(false, |t| matches!(t, Token::Combinator(_)));
                        if !last_is_combinator {
                            tokens.push(Token::Combinator(Combinator::Descendant));
                        }
                    }
                }
            }
            '>' => {
                chars.next();
                tokens.push(Token::Combinator(Combinator::Child));
            }
            '+' => {
                chars.next();
                tokens.push(Token::Combinator(Combinator::NextSibling));
            }
            '~' => {
                chars.next();
                tokens.push(Token::Combinator(Combinator::GeneralSibling));
            }
            '.' => {
                chars.next();
                let class_name = collect_identifier(&mut chars);
                tokens.push(Token::Class(class_name));
            }
            '#' => {
                chars.next();
                let id = collect_identifier(&mut chars);
                tokens.push(Token::Id(id));
            }
            '[' => {
                chars.next();
                let attr_token = parse_attribute(&mut chars)?;
                tokens.push(attr_token);
            }
            _ if ch.is_alphabetic() || ch == '*' => {
                let tag_name = collect_identifier(&mut chars);
                if tag_name != "*" {
                    tokens.push(Token::TagName(tag_name));
                }
            }
            _ => {
                return Err(format!("Unexpected character: {}", ch));
            }
        }
    }

    Ok(tokens)
}

fn collect_identifier(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut identifier = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            identifier.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    identifier
}

fn parse_attribute(chars: &mut std::iter::Peekable<std::str::Chars>) -> Result<Token, String> {
    let name = collect_identifier(chars);

    // Skip whitespace
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let (operator, value) = if let Some(&ch) = chars.peek() {
        match ch {
            ']' => {
                chars.next();
                return Ok(Token::Attribute {
                    name,
                    value: None,
                    operator: AttributeOperator::Exists,
                });
            }
            '=' | '~' | '^' | '$' | '*' => {
                let op_char = ch;
                chars.next();

                let operator = if chars.peek() == Some(&'=') {
                    chars.next();
                    match op_char {
                        '~' => AttributeOperator::WordMatch,
                        '^' => AttributeOperator::StartsWith,
                        '$' => AttributeOperator::EndsWith,
                        '*' => AttributeOperator::Contains,
                        _ => AttributeOperator::Equals,
                    }
                } else if op_char == '=' {
                    AttributeOperator::Equals
                } else {
                    return Err(format!("Invalid attribute operator"));
                };

                // Skip whitespace
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() {
                        chars.next();
                    } else {
                        break;
                    }
                }

                // Parse value (can be quoted or unquoted)
                let value = if let Some(&quote) = chars.peek() {
                    if quote == '"' || quote == '\'' {
                        chars.next();
                        let mut val = String::new();
                        while let Some(&ch) = chars.peek() {
                            if ch == quote {
                                chars.next();
                                break;
                            }
                            val.push(ch);
                            chars.next();
                        }
                        val
                    } else {
                        collect_identifier(chars)
                    }
                } else {
                    return Err("Expected attribute value".to_string());
                };

                (operator, Some(value))
            }
            _ => {
                return Err(format!(
                    "Unexpected character in attribute selector: {}",
                    ch
                ))
            }
        }
    } else {
        return Err("Unexpected end of attribute selector".to_string());
    };

    // Expect closing bracket
    if chars.next() != Some(']') {
        return Err("Expected closing bracket".to_string());
    }

    Ok(Token::Attribute {
        name,
        value,
        operator,
    })
}

fn generate_sql(tokens: &[Token]) -> Result<String, String> {
    if tokens.is_empty() {
        return Ok("SELECT * FROM nodes".to_string());
    }

    let mut sql_joins = String::from("FROM nodes n1");
    let mut join_count = 1;
    let mut where_clauses = Vec::new();
    let mut current_table = "n1".to_string();

    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::TagName(tag) => {
                where_clauses.push(format!(
                    "{}.tag_name = '{}'",
                    current_table,
                    escape_sql(tag)
                ));
            }
            Token::Class(class) => {
                join_count += 1;
                let attr_alias = format!("a{}", join_count);
                sql_joins.push_str(&format!(
                    "\nJOIN attributes {} ON {}.node_id = {}.id",
                    attr_alias, attr_alias, current_table
                ));
                where_clauses.push(format!("{}.name = 'class'", attr_alias));
                where_clauses.push(format!(
                    "({}.value = '{}' OR {}.value LIKE '% {}' OR {}.value LIKE '{} %' OR {}.value LIKE '% {} %')",
                    attr_alias, escape_sql(class),
                    attr_alias, escape_sql(class),
                    attr_alias, escape_sql(class),
                    attr_alias, escape_sql(class)
                ));
            }
            Token::Id(id) => {
                join_count += 1;
                let attr_alias = format!("a{}", join_count);
                sql_joins.push_str(&format!(
                    "\nJOIN attributes {} ON {}.node_id = {}.id",
                    attr_alias, attr_alias, current_table
                ));
                where_clauses.push(format!("{}.name = 'id'", attr_alias));
                where_clauses.push(format!("{}.value = '{}'", attr_alias, escape_sql(id)));
            }
            Token::Attribute {
                name,
                value,
                operator,
            } => {
                join_count += 1;
                let attr_alias = format!("a{}", join_count);
                sql_joins.push_str(&format!(
                    "\nJOIN attributes {} ON {}.node_id = {}.id",
                    attr_alias, attr_alias, current_table
                ));
                where_clauses.push(format!("{}.name = '{}'", attr_alias, escape_sql(name)));

                if let Some(val) = value {
                    let condition = match operator {
                        AttributeOperator::Exists => continue,
                        AttributeOperator::Equals => {
                            format!("{}.value = '{}'", attr_alias, escape_sql(val))
                        }
                        AttributeOperator::Contains => {
                            format!("{}.value LIKE '%{}%'", attr_alias, escape_sql(val))
                        }
                        AttributeOperator::StartsWith => {
                            format!("{}.value LIKE '{}%'", attr_alias, escape_sql(val))
                        }
                        AttributeOperator::EndsWith => {
                            format!("{}.value LIKE '%{}'", attr_alias, escape_sql(val))
                        }
                        AttributeOperator::WordMatch => {
                            format!(
                                "({0}.value = '{1}' OR {0}.value LIKE '% {1}' OR {0}.value LIKE '{1} %' OR {0}.value LIKE '% {1} %')",
                                attr_alias, escape_sql(val)
                            )
                        }
                    };
                    where_clauses.push(condition);
                }
            }
            Token::Combinator(combinator) => {
                if i + 1 >= tokens.len() {
                    return Err("Combinator must be followed by a selector".to_string());
                }

                join_count += 1;
                let next_table = format!("n{}", join_count);

                match combinator {
                    Combinator::Child => {
                        sql_joins.push_str(&format!(
                            "\nJOIN nodes {} ON {}.parent_id = {}.id",
                            next_table, next_table, current_table
                        ));
                    }
                    Combinator::Descendant => {
                        // Use recursive CTE for descendant relationship
                        sql_joins.push_str(&format!(
                            "\nJOIN nodes {} ON {}.id IN (
    WITH RECURSIVE descendants AS (
        SELECT id FROM nodes WHERE parent_id = {}.id
        UNION ALL
        SELECT n.id FROM nodes n
        JOIN descendants d ON n.parent_id = d.id
    )
    SELECT id FROM descendants
)",
                            next_table, next_table, current_table
                        ));
                    }
                    Combinator::NextSibling => {
                        return Err("Next sibling combinator (+) not yet supported".to_string());
                    }
                    Combinator::GeneralSibling => {
                        return Err("General sibling combinator (~) not yet supported".to_string());
                    }
                }

                current_table = next_table;
            }
        }
    }

    let mut sql = format!("SELECT DISTINCT {}.*\n{}", current_table, sql_joins);

    if !where_clauses.is_empty() {
        sql.push_str("\nWHERE ");
        sql.push_str(&where_clauses.join(" AND "));
    }

    Ok(sql)
}

fn escape_sql(s: &str) -> String {
    s.replace("'", "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple_tag() {
        let tokens = tokenize("div").unwrap();
        assert_eq!(tokens, vec![Token::TagName("div".to_string())]);
    }

    #[test]
    fn test_tokenize_class() {
        let tokens = tokenize(".container").unwrap();
        assert_eq!(tokens, vec![Token::Class("container".to_string())]);
    }

    #[test]
    fn test_tokenize_id() {
        let tokens = tokenize("#main").unwrap();
        assert_eq!(tokens, vec![Token::Id("main".to_string())]);
    }

    #[test]
    fn test_tokenize_attribute_exists() {
        let tokens = tokenize("[data-id]").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Attribute {
                name: "data-id".to_string(),
                value: None,
                operator: AttributeOperator::Exists,
            }]
        );
    }

    #[test]
    fn test_tokenize_attribute_equals() {
        let tokens = tokenize("[href='#']").unwrap();
        assert_eq!(
            tokens,
            vec![Token::Attribute {
                name: "href".to_string(),
                value: Some("#".to_string()),
                operator: AttributeOperator::Equals,
            }]
        );
    }

    #[test]
    fn test_tokenize_complex() {
        let tokens = tokenize("div.container > p#intro").unwrap();
        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0], Token::TagName(_)));
        assert!(matches!(tokens[1], Token::Class(_)));
        assert!(matches!(tokens[2], Token::Combinator(Combinator::Child)));
    }

    #[test]
    fn test_css_to_sql_simple_tag() {
        let sql = css_to_sql("div").unwrap();
        assert!(sql.contains("tag_name = 'div'"));
    }

    #[test]
    fn test_css_to_sql_class() {
        let sql = css_to_sql(".container").unwrap();
        assert!(sql.contains("name = 'class'"));
        assert!(sql.contains("value"));
    }
}
