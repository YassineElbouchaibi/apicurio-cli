// Utilities for generating output paths based on artifact metadata

use convert_case::{Case, Casing};

/// Determine file extension for a given artifact type
pub fn extension_for_type(artifact_type: &str) -> &'static str {
    match artifact_type.to_lowercase().as_str() {
        "protobuf" => "proto",
        "avro" => "avsc",
        "json" => "json",
        "openapi" => "yaml",
        "asyncapi" => "yaml",
        "graphql" => "graphql",
        "xml" => "xsd",
        "wsdl" => "wsdl",
        _ => "txt",
    }
}

/// Expand an output pattern using group/artifact/version and extension
pub fn expand_pattern(
    pattern: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
    ext: &str,
) -> String {
    let mut result = pattern.to_string();
    result = result.replace("{groupId}", group_id);
    result = result.replace("{artifactId}", artifact_id);
    result = result.replace("{version}", version);
    result = result.replace("{ext}", ext);

    let artifact_parts: Vec<&str> = artifact_id.split('.').collect();

    if result.contains("{artifactId.path}") {
        let path_version = if artifact_parts.len() > 1 {
            artifact_parts[..artifact_parts.len() - 1].join("/")
        } else {
            String::new()
        };
        result = result.replace("{artifactId.path}", &path_version);
    }

    if result.contains("{artifactId.fullPath}") {
        let full_path_version = artifact_parts.join("/");
        result = result.replace("{artifactId.fullPath}", &full_path_version);
    }

    if result.contains("{artifactId.snake_case}") {
        let snake_case = artifact_id.replace('.', "_").to_lowercase();
        result = result.replace("{artifactId.snake_case}", &snake_case);
    }

    if result.contains("{artifactId.kebab_case}") {
        let kebab_case = artifact_id.replace('.', "-").to_lowercase();
        result = result.replace("{artifactId.kebab_case}", &kebab_case);
    }

    if result.contains("{artifactId.lowercase}") {
        result = result.replace("{artifactId.lowercase}", &artifact_id.to_lowercase());
    }

    if result.contains("{artifactId.last}") {
        let last_part = artifact_parts.last().unwrap_or(&artifact_id);
        result = result.replace("{artifactId.last}", last_part);
    }

    if result.contains("{artifactId.lastLowercase}") {
        let last_part = artifact_parts.last().unwrap_or(&artifact_id).to_lowercase();
        result = result.replace("{artifactId.lastLowercase}", &last_part);
    }

    if result.contains("{artifactId.lastSnakeCase}") {
        let last_part = artifact_parts.last().unwrap_or(&artifact_id);
        let snake_case_part = last_part.to_case(Case::Snake);
        result = result.replace("{artifactId.lastSnakeCase}", &snake_case_part);
    }

    for (i, part) in artifact_parts.iter().enumerate() {
        let placeholder = format!("{{artifactParts[{i}]}}");
        result = result.replace(&placeholder, part);
    }

    result
}

/// Generate an output path using an output pattern
pub fn generate_output_path(
    pattern: &str,
    group_id: &str,
    artifact_id: &str,
    version: &str,
    artifact_type: &str,
) -> String {
    let ext = extension_for_type(artifact_type);
    expand_pattern(pattern, group_id, artifact_id, version, ext)
}
