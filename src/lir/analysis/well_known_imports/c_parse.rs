#[derive(Debug)]
pub struct FunctionDecl {
    pub return_type: String,
    pub name: String,
    pub params: Vec<Param>,
    pub is_variadic: bool,
}

#[derive(Debug)]
pub struct Param {
    pub ty: String,
    pub name: String,
}

pub fn parse_c_function_decl(input: &str) -> Result<FunctionDecl, String> {
    let input = input.trim().trim_end_matches(';').trim();

    // Split into "RET NAME" and "(params)"
    let open_paren = input.find('(').ok_or("Missing '('")?;
    let close_paren = input.rfind(')').ok_or("Missing ')'")?;

    let header = input[..open_paren].trim();
    let params_str = input[open_paren + 1..close_paren].trim();

    // Split header into return type and function name
    let mut header_parts = header.rsplitn(2, char::is_whitespace);
    let name = header_parts
        .next()
        .ok_or("Missing function name")?
        .to_string();

    let return_type = header_parts
        .next()
        .ok_or("Missing return type")?
        .to_string();

    let mut params = Vec::new();
    let mut is_variadic = false;

    if !params_str.is_empty() && !matches!(params_str, "void" | "VOID" | "voidfpu") {
        for param in params_str.split(',') {
            let param = param.trim();

            if param == "..." {
                is_variadic = true;
                continue;
            }

            let mut parts = param.rsplitn(2, char::is_whitespace);
            let name = parts.next().ok_or("Missing parameter name")?.to_string();

            let ty = parts.next().ok_or("Missing parameter type")?.to_string();

            params.push(Param { ty, name });
        }
    }

    Ok(FunctionDecl {
        return_type,
        name,
        params,
        is_variadic,
    })
}
