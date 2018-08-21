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

#![crate_name = "osm_microbenchmarks"]
#![crate_type = "staticlib"]

#![cfg_attr(not(target_env = "sgx"), no_std)]
#![cfg_attr(target_env = "sgx", feature(rustc_private))]

extern crate sgx_types;
#[cfg(not(target_env = "sgx"))]
extern crate sgx_tstd as std;

extern crate osm;
extern crate path_oram;
extern crate generic_array;

use generic_array::typenum::U160;
use osm::{OsmClient, STDOsmClient};
use path_oram::{LocalServer, PathDOramClient};

use sgx_types::*;

type Key = u64;
type Value = u64;

#[no_mangle]
pub extern "C" fn osm_search(osm_client_ref: usize, server_ref: usize, key_ref: usize, range: usize) -> sgx_status_t {

    let osm_client = unsafe {
        let osm_client = osm_client_ref as *mut STDOsmClient<Key, Value, PathDOramClient<U160>>;
        &(*osm_client)
    };

    let server = unsafe {
        let server = server_ref as *mut LocalServer<PathDOramClient<U160>>;
        &mut(*server)
    };

    let ref mut read_key = unsafe {
        let key = key_ref as *const Key;
        *key
    };

    let mut osm_client = osm_client.clone();
    let num_reads = 2000;
    for _ in 0..num_reads {
        osm_client.search(&read_key, 0, range, server).unwrap();
    }

    sgx_status_t::SGX_SUCCESS
}
