use std::{convert::Infallible, path::Path};

use itertools::Itertools;
use rig::{
    client::{AgentClientExt, Nothing},
    completion::TypedPrompt,
    providers::{anthropic, ollama},
    tool::Tool,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    Ctxt, Log, StartingLocation,
    utils::{find_file_path_from_file_name, validate_file_path},
};

pub fn parse_using_agent_ollama(ctx: &Ctxt, log: &Log) -> anyhow::Result<Vec<StartingLocation>> {
    match &ctx.ollama_url {
        Some(url) => {
            let rt = tokio::runtime::Runtime::new()?;
            let sls = rt.block_on(parse_ollama(ctx, log, url))?;
            Ok(sls)
        }
        None => Ok(vec![]),
    }
}

async fn parse_ollama(ctx: &Ctxt, log: &Log, url: &str) -> anyhow::Result<Vec<StartingLocation>> {
    let client = ollama::Client::builder()
        .api_key(Nothing)
        .base_url(url)
        .build()?;

    let batch_size = 1000;
    let mut ret = vec![];
    for (index, batch) in log
        .iter()
        .chunks(batch_size)
        .into_iter()
        .map(|chunk| chunk.fold(String::new(), |acc, x| format!("{}\n{}", acc, x)))
        .enumerate()
    {
        let result :Vec<StartingLocation> = client
        .agent("qwen3.8:27b")
        .tool(FindFilePath{src_path:ctx.src_path.clone()})
        .tool(ValidateFilePath{src_path:ctx.src_path.clone()})
        .max_tokens(1024 * 1024)
        .default_max_turns(1024 * 1024)
        .build()

        .prompt_typed(format!("{}", format!(r#"
            "new to old log line. You are currently in range: {l}~{r}
            ""
            "{logs}"
            ""
            You have some program's log file that is just crashed.
            You have to find suspicious log line and extract source locations for potential bug line.
            You can make multiple locations.
            Evaluate your confidence of each locations ranges 0.0~1.0. specify line number of suspicious line.
            Write down description of why you think that line is suspicious.
            word like "panic" or "fatal" is a good hint.
            If can't find anything, write down the reason as error message verbosely.
            You cannot see file content. Juse use file name and line number in log line.
            You must validate your file path using ValidateFilePath before submit. If it returns false, use FindFilePath again.
            "#,l = index * batch_size,r = std::cmp::min((index +1) * batch_size - 1, log.lines.len()) ,logs = batch))).await?;

        tracing::debug!("result: {:?}", result);
        ret.extend(
            result
                .into_iter()
                .filter(|x| validate_file_path(&x.loc.file_path, &Path::new(&ctx.src_path))),
        );
    }

    tracing::debug!("ret: {:?}", ret);
    Ok(ret)
}

pub fn parse_using_agent_anthropic(ctx: &Ctxt, log: &Log) -> anyhow::Result<Vec<StartingLocation>> {
    match &ctx.anthropic {
        Some(api_key) => {
            let rt = tokio::runtime::Runtime::new()?;
            let sls = rt.block_on(parse_anthropic(ctx, log, api_key))?;
            Ok(sls)
        }
        None => Ok(vec![]),
    }
}

async fn parse_anthropic(
    ctx: &Ctxt,
    log: &Log,
    api_key: &str,
) -> anyhow::Result<Vec<StartingLocation>> {
    let client = anthropic::Client::builder().api_key(api_key).build()?;

    let batch_size = 1000;
    let mut ret = vec![];
    for (index, batch) in log
        .iter()
        .chunks(batch_size)
        .into_iter()
        .map(|chunk| chunk.fold(String::new(), |acc, x| format!("{}\n{}", acc, x)))
        .enumerate()
    {
        let result :Vec<StartingLocation> = client
        .agent(anthropic::completion::CLAUDE_SONNET_4_6)
        .tool(FindFilePath{src_path:ctx.src_path.clone()})
        .tool(ValidateFilePath{src_path:ctx.src_path.clone()})
        .max_tokens(1024 * 1024)
        .default_max_turns(1024 * 1024)
        .build()

        .prompt_typed(format!("{}", format!(r#"
            "new to old log line. You are currently in range: {l}~{r}
            ""
            "{logs}"
            ""
            You have some program's log file that is just crashed.
            You have to find suspicious log line and extract source locations for potential bug line.
            You can make multiple locations.
            Evaluate your confidence of each locations ranges 0.0~1.0. specify line number of suspicious line.
            Write down description of why you think that line is suspicious.
            word like "panic" or "fatal" is a good hint.
            If can't find anything, write down the reason as error message verbosely.
            You cannot see file content. Juse use file name and line number in log line.
            You must validate your file path using ValidateFilePath before submit. If it returns false, use FindFilePath again.
            "#,l = index * batch_size,r = std::cmp::min((index +1) * batch_size - 1, log.lines.len()) ,logs = batch))).await?;

        tracing::debug!("result: {:?}", result);
        ret.extend(
            result
                .into_iter()
                .filter(|x| validate_file_path(&x.loc.file_path, &Path::new(&ctx.src_path))),
        );
    }

    tracing::debug!("ret: {:?}", ret);
    Ok(ret)
}

#[derive(Deserialize)]
struct FindFilePath {
    src_path: String,
}
#[derive(Deserialize)]
struct FindFilePathArgs {
    file_name: String,
}
impl Tool for FindFilePath {
    const NAME: &'static str = "find_file_path";

    type Error = Infallible;
    type Args = FindFilePathArgs;
    type Output = Vec<String>;

    fn description(&self) -> String {
        "Find full file path using file name and source path".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_name": {
                    "type": "string",
                    "description": "The file name"
                },
            },
            "required": ["file_name"],
        })
    }

    async fn call(
        &self,
        _context: &mut rig::prelude::ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        let paths = find_file_path_from_file_name(&args.file_name, Path::new(&self.src_path));

        Ok(paths)
    }
}

#[derive(Deserialize)]
struct ValidateFilePath {
    src_path: String,
}
#[derive(Deserialize)]
struct ValidateFilePathArgs {
    file_path: String,
}
impl Tool for ValidateFilePath {
    const NAME: &'static str = "validate_file_path";

    type Error = Infallible;
    type Args = ValidateFilePathArgs;
    type Output = bool;

    fn description(&self) -> String {
        "Validate whether provided file path is existing or not".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_name": {
                    "type": "string",
                    "description": "The file path"
                },
            },
            "required": ["file_path"],
        })
    }

    async fn call(
        &self,
        _context: &mut rig::prelude::ToolContext,
        args: Self::Args,
    ) -> Result<Self::Output, Self::Error> {
        Ok(validate_file_path(
            &args.file_path,
            &Path::new(&self.src_path),
        ))
    }
}
