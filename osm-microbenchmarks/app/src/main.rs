// Copyright (C) 2017-2018 Baidu, Inc. All Rights Reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//  * Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//  * Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in
//    the documentation and/or other materials provided with the
//    distribution.
//  * Neither the name of Baidu, Inc., nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

extern crate sgx_types;
extern crate sgx_urts;
extern crate generic_array;

extern crate path_oram;
extern crate osm;
extern crate rand;
extern crate time;
#[macro_use]
extern crate structopt;
extern crate pretty_env_logger;
extern crate dirs;

use structopt::StructOpt;

use sgx_types::*;
use sgx_urts::SgxEnclave;

use std::io::{Read, Write};
use std::fs;
use std::path;

mod microbenchmarks;

#[derive(StructOpt, Debug)]
/// Run Signal benchmarks
#[structopt(name = "signal")]
struct Signal {
    #[structopt(help = "Initial size of the storage", default_value = "1024")]
    initial_size: usize
}

#[derive(StructOpt, Debug)]
/// Run Key Transparency benchmarks
#[structopt(name = "kt")]
struct KeyTransparency {
    #[structopt(help = "Initial size of the storage", default_value = "1024")]
    initial_size: usize
}

#[derive(StructOpt, Debug)]
/// Run Searchable Encryption benchmarks
#[structopt(name = "se")]
struct SearchableEncryption {
    #[structopt(help = "Path to index", default_value = "enron-index.json")]
    index: String,
    #[structopt(help = "Number of results to return", default_value = "10")]
    number_of_results: usize,
    #[structopt(help = "Number of documents to insert", default_value = "10")]
    number_of_documents: usize,
}

#[derive(StructOpt, Debug)]
struct OsmCommand {
    #[structopt(subcommand)]
    osm: OsmMicrobenchmarks
}

#[derive(StructOpt, Debug)]
/// Run DOSM microbenchmarks
#[structopt(name = "dosm")]
enum OsmMicrobenchmarks {

    #[structopt(name = "range")]
    Range,

    #[structopt(name = "insert-many")]
    InsertMany,

    #[structopt(name = "insert-one")]
    InsertOne { 
        #[structopt(help = "Number of keys to insert", default_value = "1")]
        number_of_keys_to_insert: usize, 
        #[structopt(help = "Initial size of the storage", default_value = "1024")]
        initial_size: usize
    },

    #[structopt(name = "delete-one")]
    DeleteOne { 
        #[structopt(help = "Number of keys to delete", default_value = "1")]
        number_of_keys_to_delete: usize,
        #[structopt(help = "Initial size of the storage", default_value = "1024")]
        initial_size: usize,
    },
}

#[derive(StructOpt, Debug)]
struct OramCommand {
    #[structopt(subcommand)]
    oram: OramMicrobenchmarks
}

#[derive(StructOpt, Debug)]
/// Run DORAM microbenchmarks
#[structopt(name = "doram")]
enum OramMicrobenchmarks {
    
    #[structopt(name = "zerotrace")]
    ZeroTrace { 
        #[structopt(help = "Initial size of the storage", default_value = "1024")]
        initial_size: usize,
    },
    #[structopt(name = "access")]
    OramAccess { 
        #[structopt(help = "Initial size of the storage", default_value = "1024")]
        initial_size: usize,
        #[structopt(help = "Block size", default_value = "160")]
        block_size: usize,
    },
}

#[derive(StructOpt, Debug)]
enum OptionsCommand {
    #[structopt(name = "osm")]
    Osm(OsmCommand),
    #[structopt(name = "oram")]
    Oram(OramCommand),
    #[structopt(name = "se")]
    SE(SearchableEncryption),
    #[structopt(name = "signal")]
    Signal(Signal),
    #[structopt(name = "kt")]
    KT(KeyTransparency),
}

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(subcommand)]
    options: OptionsCommand
}


static ENCLAVE_FILE: &'static str = "enclave.signed.so";
static ENCLAVE_TOKEN: &'static str = "enclave.token";

fn init_enclave() -> SgxResult<SgxEnclave> {
    
    let mut launch_token: sgx_launch_token_t = [0; 1024];
    let mut launch_token_updated: i32 = 0;
    // Step 1: try to retrieve the launch token saved by last transaction 
    //         if there is no token, then create a new one.
    // 
    // try to get the token saved in $HOME */
    let mut home_dir = path::PathBuf::new();
    let use_token = match dirs::home_dir() {
        Some(path) => {
            println!("[+] Home dir is {}", path.display());
            home_dir = path;
            true
        },
        None => {
            println!("[-] Cannot get home dir");
            false
        }
    };

    let token_file: path::PathBuf = home_dir.join(ENCLAVE_TOKEN);;
    if use_token == true {
        match fs::File::open(&token_file) {
            Err(_) => {
                println!("[-] Open token file {} error! Will create one.", token_file.as_path().to_str().unwrap());
            },
            Ok(mut f) => {
                println!("[+] Open token file success! ");
                match f.read(&mut launch_token) {
                    Ok(1024) => {
                        println!("[+] Token file valid!");
                    },
                    _ => println!("[+] Token file invalid, will create new token file"),
                }
            }
        }
    }

    // Step 2: call sgx_create_enclave to initialize an enclave instance
    // Debug Support: set 2nd parameter to 1 
    let debug = 1;
    let mut misc_attr = sgx_misc_attribute_t {secs_attr: sgx_attributes_t { flags:0, xfrm:0}, misc_select:0};
    let enclave = try!(SgxEnclave::create(ENCLAVE_FILE, 
                                          debug, 
                                          &mut launch_token,
                                          &mut launch_token_updated,
                                          &mut misc_attr));
    
    // Step 3: save the launch token if it is updated 
    if use_token == true && launch_token_updated != 0 {
        // reopen the file with write capablity 
        match fs::File::create(&token_file) {
            Ok(mut f) => {
                match f.write_all(&launch_token) {
                    Ok(()) => println!("[+] Saved updated launch token!"),
                    Err(_) => println!("[-] Failed to save updated launch token!"),
                }
            },
            Err(_) => {
                println!("[-] Failed to save updated enclave token, but doesn't matter");
            },
        }
    }

    Ok(enclave)
}

fn main() { 
    let options = Options::from_args();
    let enclave = match init_enclave() {
        Ok(r) => {
            println!("[+] Init Enclave Successful {}!", r.geteid());
            r
        },
        Err(x) => {
            println!("[-] Init Enclave Failed {}!", x.as_str());
            return;
        },
    };
    let result = match options.options {
        OptionsCommand::Osm(inner) => {
            match inner.osm {
                OsmMicrobenchmarks::Range => {
                    println!("Running osm::range: \n
                             Retrieving 10 results for 2^24 key-value pairs, with 1 - 2^8 values per key \n");
                    let mut actual_result = 0;
                    for i in 16..24 {
                        actual_result += microbenchmarks::search(&enclave, 1 << i, 1 << (24 - i), 10).from_key();
                    }
                    println!("\n----------------------------\n");
                    println!("Retrieving 1, 10, 20, ..., 60 results for 2^24 key-value pairs, with 2^10 values per key \n");
                    for i in vec![1, 10, 20, 30, 40, 50, 60] {
                        actual_result += microbenchmarks::search(&enclave, 1 << (24 - 10), 1 << 10, i).from_key();
                    }
                    sgx_status_t::from_repr(actual_result).unwrap()

                }
                OsmMicrobenchmarks::InsertMany => {
                    println!("Running osm::insert_many:\n
                             Inserting 100 items into storage of size 2^16 - 2^25");
                    let mut actual_result = 0;
                    for i in 16..25 {
                        // If the operation is successful, it returns 0.
                        actual_result += microbenchmarks::insert_many(&enclave, 1 << i, 100).from_key();
                    }
                    println!("\n----------------------------\n");
                    sgx_status_t::from_repr(actual_result).unwrap()
                }
                OsmMicrobenchmarks::InsertOne { number_of_keys_to_insert, initial_size} => {
                    println!("Running osm::insert_one");
                    let result = microbenchmarks::insert_one(&enclave, initial_size, number_of_keys_to_insert);
                    println!("\n----------------------------\n");
                    result

                }
                OsmMicrobenchmarks::DeleteOne { number_of_keys_to_delete, initial_size} => {
                    println!("Running osm::insert_one");
                    let result = microbenchmarks::delete_one(&enclave, initial_size, number_of_keys_to_delete);
                    println!("\n----------------------------\n");
                    result
                }
            }
        }
        OptionsCommand::Oram(inner) => {
            match inner.oram {
                OramMicrobenchmarks::ZeroTrace { initial_size } => {
                    println!("Running ZeroTrace");
                    let result = microbenchmarks::zerotrace(&enclave, initial_size);
                    println!("\n----------------------------\n");
                    result
                }
                OramMicrobenchmarks::OramAccess { block_size, initial_size } => {
                    println!("Running DORAM Access");
                    println!("\nItems: {}, Blocksize: {}", initial_size, block_size);
                    let result = microbenchmarks::doram(&enclave, initial_size, block_size as _);
                    println!("\n----------------------------\n");
                    result
                }
            }
        }
        OptionsCommand::SE(inner) => {
            // TODO
            // println!("Running SE benchmarks on the Enron dataset (specifically `kaminski-v`): average of 100 measurements");
            // let index_location = inner.index;
            // let number_of_results = inner.number_of_results;
            // let number_of_documents = inner.number_of_documents;
            // let result1 = enron::search(&enclave, &file, number_of_results).from_key();
            // let result2 = enron::insert(&enclave, &file, number_of_documents).from_key();
            // sgx_status_t::from_repr(result1 + result2).unwrap()
            unimplemented!()
        }
        OptionsCommand::Signal(inner) => {
            // TODO
            // println!("Running Signal benchmarks");
            // let mut i = inner.initial_size;
            // let mut actual_result = 0;
            // while i >= 10000 {
            //     let inner_result = signal::run(&enclave, i);
            //     actual_result += result.from_key();
            //     i /= 2;
            // }
            // sgx_status_t::from_repr(actual_result).unwrap()
            unimplemented!()
        }
        OptionsCommand::KT(inner) => {
            // TODO
            // println!("Running Key Transparency benchmarks");
            // let mut i = inner.initial_size;
            // let mut actual_result = 0;
            // while i >= 10000 {
            //     let inner_result = key_transparency::run(&enclave, i);
            //     actual_result += result.from_key();
            //     i /= 2;
            // }
            // sgx_status_t::from_repr(actual_result).unwrap()
            unimplemented!()
        }
    };


    match result {
        sgx_status_t::SGX_SUCCESS => {},
        _ => {
            println!("[-] ECALL Enclave Failed {}!", result.as_str());
            return;
        }
    }

    println!("[+] osm_search success...");
    
    enclave.destroy();
}
