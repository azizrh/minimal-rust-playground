use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
use uuid::Uuid;
use log::{info, error};

#[derive(Deserialize)]
struct CodeRequest {
    code: String,
}

#[derive(Serialize)]
struct CodeResponse {
    success: bool,
    output: String,
    error: String,
}

async fn execute_rust_code(code_req: web::Json<CodeRequest>) -> impl Responder {
    info!("Received code execution request");
    
    // Create a temporary directory
    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(e) => {
            error!("Failed to create temporary directory: {}", e);
            return HttpResponse::InternalServerError().json(CodeResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to create temporary directory: {}", e),
            });
        }
    };
    
    let file_id = Uuid::new_v4();
    let file_path = temp_dir.path().join(format!("{}.rs", file_id));
    
    // Write the code to a file
    let mut file = match File::create(&file_path) {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to create file: {}", e);
            return HttpResponse::InternalServerError().json(CodeResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to create file: {}", e),
            });
        }
    };
    
    if let Err(e) = file.write_all(code_req.code.as_bytes()) {
        error!("Failed to write to file: {}", e);
        return HttpResponse::InternalServerError().json(CodeResponse {
            success: false,
            output: String::new(),
            error: format!("Failed to write to file: {}", e),
        });
    }
    
    // Compile the Rust code
    let compile_output = Command::new("rustc")
        .arg(&file_path)
        .arg("-o")
        .arg(temp_dir.path().join(file_id.to_string()))
        .output();
    
    match compile_output {
        Ok(output) => {
            if !output.status.success() {
                let error_message = String::from_utf8_lossy(&output.stderr).to_string();
                error!("Compilation error: {}", error_message);
                return HttpResponse::Ok().json(CodeResponse {
                    success: false,
                    output: String::new(),
                    error: error_message,
                });
            }
            
            // Execute the compiled binary
            let execution_output = Command::new(temp_dir.path().join(file_id.to_string()))
                .output();
            
            match execution_output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    
                    if output.status.success() {
                        info!("Code executed successfully");
                        HttpResponse::Ok().json(CodeResponse {
                            success: true,
                            output: stdout,
                            error: stderr,
                        })
                    } else {
                        info!("Runtime error occurred");
                        HttpResponse::Ok().json(CodeResponse {
                            success: false,
                            output: stdout,
                            error: stderr,
                        })
                    }
                },
                Err(e) => {
                    error!("Failed to execute program: {}", e);
                    HttpResponse::InternalServerError().json(CodeResponse {
                        success: false,
                        output: String::new(),
                        error: format!("Failed to execute program: {}", e),
                    })
                }
            }
        },
        Err(e) => {
            error!("Failed to compile program: {}", e);
            HttpResponse::InternalServerError().json(CodeResponse {
                success: false,
                output: String::new(),
                error: format!("Failed to compile program: {}", e),
            })
        }
    }
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Rust Playground API is running")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Rust Playground server");
    
    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        
        App::new()
            .wrap(cors)
            .route("/health", web::get().to(health_check))
            .route("/execute", web::post().to(execute_rust_code))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
