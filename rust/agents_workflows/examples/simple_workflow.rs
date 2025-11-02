// Copyright (c) Microsoft. All rights reserved.

//! Simple workflow example demonstrating basic functionality

use agents_workflows::{
    ExecutorIsh, ExecutorRegistration, FunctionExecutor, WorkflowBuilder,
};
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Simple Workflow Example");

    // Create a simple echo executor
    let echo_registration = ExecutorRegistration::new(
        "echo".to_string(),
        "EchoExecutor".to_string(),
        || {
            Ok(Box::new(FunctionExecutor::new(
                "echo".to_string(),
                |input: Value| {
                    println!("Echo received: {}", input);
                    Ok(format!("Echo: {}", input))
                },
            )))
        },
    );

    // Create a simple transform executor
    let transform_registration = ExecutorRegistration::new(
        "transform".to_string(),
        "TransformExecutor".to_string(),
        || {
            Ok(Box::new(FunctionExecutor::new(
                "transform".to_string(),
                |input: Value| {
                    if let Value::String(s) = input {
                        Ok(s.to_uppercase())
                    } else {
                        Ok("TRANSFORMED".to_string())
                    }
                },
            )))
        },
    );

    // Build the workflow
    let echo_executor = ExecutorIsh::bound(echo_registration);
    let transform_executor = ExecutorIsh::bound(transform_registration);

    let workflow = WorkflowBuilder::new(echo_executor.clone())?
        .with_name("Simple Echo Workflow")
        .with_description("A simple workflow that echoes and transforms input")
        .add_edge(echo_executor, transform_executor.clone())?
        .with_output_from(&[transform_executor])?
        .build()?;

    println!(" Workflow built successfully!");
    println!("   - Start executor: {}", workflow.start_executor_id());
    println!("   - Name: {}", workflow.name().unwrap_or("Unknown"));
    println!("   - Description: {}", workflow.description().unwrap_or("None"));

    // Get workflow info
    let workflow_info = workflow.to_workflow_info();
    println!("   - Executors: {:?}", workflow_info.executor_ids);
    println!("   - Allows concurrent: {}", workflow_info.allows_concurrent);

    // Get edges info
    let edges = workflow.reflect_edges();
    println!("   - Edges: {} sources", edges.len());
    for (source, edge_list) in edges {
        println!("     {} -> {} edges", source, edge_list.len());
    }

    println!("🎉 Workflow example completed successfully!");

    Ok(())
}