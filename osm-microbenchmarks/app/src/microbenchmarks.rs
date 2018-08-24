use generic_array::typenum::{U8, U16, U32, U64, U128, U160, U256, U512};
use generic_array::ArrayLength;

use rand;
use time;
use osm::{OsmClient, STDOsmClient};
use path_oram::{LocalServer, PathDOramClient, doubly_oblivious::position_map::PositionMap, OramKey, OramPos, NoPos};
use path_oram::{TreeOramClient, BlockContent, EncN, EncBlkSize};
use path_oram::oram_crypto::{Encryptor, MerkleTree};
use rand::{OsRng, Rng};

use sgx_types::*;
use sgx_urts::SgxEnclave;
use pretty_env_logger;

type Key = u64;
type Value = u64;



extern {
    fn osm_search(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        osm_client_ref: usize,
        server_ref: usize,
        key_ref: usize,
        range: usize
    ) -> sgx_status_t;

    fn osm_insert_many(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        osm_client_ref: usize,
        server_ref: usize,
        key_ref: usize,
        key_len: usize,
        vals_ref: usize,
        vals_len: usize
    ) -> sgx_status_t;

    fn osm_insert_one(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        osm_client_ref: usize,
        server_ref: usize,
        key_ref: usize,
        value_ref: usize
    ) -> sgx_status_t;

    fn osm_delete_one(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        osm_client_ref: usize,
        server_ref: usize,
        key_ref: usize,
        value_ref: usize
    ) -> sgx_status_t;

    fn oram_zerotrace(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        oram_client_ref: usize,
        server_ref: usize,
        key_and_pos_ref: usize,
        key_and_pos_len: usize
    ) -> sgx_status_t;

    fn oram_access(
        eid: sgx_enclave_id_t,
        retval: *mut sgx_status_t,
        oram_client_ref: usize,
        server_ref: usize,
        key_and_pos_ref: usize,
        key_and_pos_len: usize,
        block_size: usize,
    ) -> sgx_status_t;
}

pub fn insert_many(enclave: &SgxEnclave, init_size: usize, n_keys: usize) -> sgx_status_t {
    println!(
        "\n[+] Size: {}, Number of keys: {}",
        init_size, n_keys
    );
    let mut map = Vec::with_capacity(n_keys);
    for _ in 0..init_size {
        let key = rand::random::<Key>();
        let value = rand::random::<Value>();
        map.push((key, value));

    }
    println!("[+] Done with map");

    let (osm_client, mut server) =
        STDOsmClient::<Key, Value, PathDOramClient<U160>>::setup(map.len() * 2, map)
            .unwrap();
    println!("[+] Done with setup");

    let mut keys = Vec::with_capacity(n_keys);
    let mut vals = Vec::with_capacity(n_keys);
    for _ in 0..n_keys {
        let key = rand::random::<Key>();
        let val = rand::random::<Value>();
        keys.push(key);
        vals.push(val);
    }

    // let mut rng = OsRng::new().unwrap();
    // let read_key = rng.choose(&keys).unwrap();
    // Stash warm-up
    // for _ in 0..30000 {
        // let _ = osm_client.search(&read_key, 0, 1, &mut server);
    // }


    // *****
    // *****
    // *****
    // Part inside here should be executed in the enclave.
    let osm_client_ref = &osm_client as *const STDOsmClient<_, _, _> as u64;
    let server_ref = &mut server as *mut LocalServer<PathDOramClient<U160>> as u64;

    //println!("Loaded enclave.");
    let read_start = time::precise_time_s();
    let mut retval = sgx_status_t::SGX_SUCCESS;
    let result = unsafe {
        osm_insert_many(
            enclave.geteid(),
            &mut retval,
            osm_client_ref as usize,
            server_ref as usize,
            keys.as_ptr() as u64 as usize,
            keys.len(),
            vals.as_ptr() as u64 as usize,
            vals.len(),
        )
    };

    let read_stop = time::precise_time_s();
    let avg_time = (read_stop - read_start) / n_keys as f64;
    println!(
        "[+] Capacity: {}, Inserted keys: {}, avg. time (s): {}",
        init_size * 2,
	    n_keys,
        avg_time
    );
    result
    // *****
    // *****
    // *****
}

pub fn insert_one(enclave: &SgxEnclave, init_size: usize, n_keys: usize) -> sgx_status_t {
    println!(
        "\n[+] Size: {}, Number of keys: {}",
        init_size, n_keys
    );
    let mut map = Vec::with_capacity(init_size);
    for _ in 0..init_size {
        let key = rand::random::<Key>();
        let value = rand::random::<Value>();
        map.push((key, value));

    }
    println!("Done with map");

    let (osm_client, mut server) =
        STDOsmClient::<Key, Value, PathDOramClient<U160>>::setup(init_size, map)
            .unwrap();
    println!("Done with setup");

    let mut keys = Vec::with_capacity(n_keys);
    let mut vals = Vec::with_capacity(n_keys);
    for _ in 0..n_keys {
        let key = rand::random::<Key>();
        let val = rand::random::<Value>();
        keys.push(key);
        vals.push(val);
    }

    // let mut rng = OsRng::new().unwrap();
    // let read_key = rng.choose(&keys).unwrap();
    // Stash warm-up
    // for _ in 0..30000 {
    //     let _ = osm_client.search(&read_key, 0, 1, &mut server);
    // }


    // *****
    // *****
    // *****
    // Part inside here should be executed in the enclave.
    let osm_client_ref = &osm_client as *const STDOsmClient<_, _, _> as usize;
    let server_ref = &mut server as *mut LocalServer<PathDOramClient<U160>> as usize;
    let mut times = Vec::<f64>::with_capacity(n_keys);
    let mut result = sgx_status_t::SGX_SUCCESS;
    let mut retval = sgx_status_t::SGX_SUCCESS;
    for (k, v) in keys.iter().zip(vals.iter()) {
        let key_ref = k as *const Key as u64;
        let val_ref = v as *const Value as u64;

        //println!("Loaded enclave.");
        let read_start = time::precise_time_s();
        //println!("Started reading");
        result = unsafe { osm_insert_one(
            enclave.geteid(),
            &mut retval,
            osm_client_ref,
            server_ref,
            key_ref as usize,
            val_ref as usize,
        )};
        let read_stop = time::precise_time_s();
        times.push(read_stop - read_start);
    }
    println!(
        "\nSize: {}, Keys: {}, times (s): {:?}",
        init_size, n_keys, times
    );
    // *****
    // *****
    // *****
    result
}

pub fn delete_one(enclave: &SgxEnclave, init_size: usize, n_keys: usize) -> sgx_status_t {

    let mut map = Vec::with_capacity(init_size);
    let mut keys = Vec::with_capacity(n_keys);
    let mut vals = Vec::with_capacity(n_keys);
    for _ in 0..init_size {
        let key = rand::random::<Key>();
        let value = rand::random::<Value>();
        map.push((key.clone(), value.clone()));
        if keys.len() < n_keys {
            keys.push(key);
            vals.push(value);
        }
    }
    println!("Done with map");

    let (osm_client, mut server) =
        STDOsmClient::<Key, Value, PathDOramClient<U160>>::setup(init_size, map)
            .unwrap();
    println!("Done with setup");

    // let mut rng = OsRng::new().unwrap();
    // let read_key = rng.choose(&keys).unwrap();
    // Stash warm-up
    // for _ in 0..30000 {
    //     let _ = osm_client.search(&read_key, 0, 1, &mut server);
    // }

    // *****
    // *****
    // *****
    // Part inside here should be executed in the enclave.
    let osm_client_ref = &osm_client as *const STDOsmClient<_, _, _> as u64;
    let server_ref = &mut server as *mut LocalServer<PathDOramClient<U160>> as u64;
    let mut times = Vec::<f64>::with_capacity(n_keys);
    let mut result = sgx_status_t::SGX_SUCCESS;
    for (k, v) in keys.iter().zip(vals.iter()) {
        let key_ref = k as *const Key as u64;
        let val_ref = v as *const Value as u64;

        println!("Loaded enclave.");
        let mut retval = sgx_status_t::SGX_SUCCESS;
        let read_start = time::precise_time_s();
        result = unsafe {
            osm_delete_one(
                enclave.geteid(),
                &mut retval,
                osm_client_ref as usize,
                server_ref as usize,
                key_ref as usize,
                val_ref as usize,
            )
        };
        let read_stop = time::precise_time_s();
        times.push(read_stop - read_start);
    }
    println!(
        "\nSize: {}, Keys: {}, times (s): {:?}",
        init_size, n_keys, times
    );
    result
    // *****
    // *****
    // *****
}

pub fn search(enclave: &SgxEnclave, n_keys: usize, vals_per_key: usize, range: usize) -> sgx_status_t {
    println!(
        "\n[+] Size: {}, Values per key: {}, range: {}",
        n_keys, vals_per_key, range
    );

    let mut all_keys = Vec::with_capacity(n_keys);
    let mut map = Vec::<(Key, Value)>::with_capacity(n_keys * vals_per_key);
    for _ in 0..n_keys {
        let key = rand::random::<Key>();
        for _ in 0..vals_per_key {
            let value = rand::random::<Value>();
            all_keys.push(key);
            map.push((key, value));
        }
    }
    println!("[+] Done with map");

    let mut rng = OsRng::new().unwrap();
    let read_key: &u64 = rng.choose(&all_keys).unwrap();

    let l = map.len();
    let (osm_client, mut server) =
        STDOsmClient::<Key, Value, PathDOramClient<U160>>::setup(map.len(), map)
            .unwrap();
    println!("[+] Done with setup: {}", l);

    // Stash warm-up
    // for _ in 0..30000 {
    //     let _ = osm_client.search(&read_key, 0, 1, &mut server);
    // }
    // println!("[+] Done with warm-up");

    // *****
    // *****
    // *****
    // Part inside here should be executed in the enclave.
    let osm_client_ref = &osm_client as *const STDOsmClient<_, _, _> as u64;
    let server_ref = &mut server as *mut LocalServer<PathDOramClient<U160>> as u64;
    let key_ref = read_key as *const Key as u64;

    let num_reads: usize = 2000;

    let read_start = time::precise_time_s();

    let mut retval = sgx_status_t::SGX_SUCCESS;
    let result = unsafe {
        osm_search(
            enclave.geteid(),
            &mut retval,
            osm_client_ref as usize,
            server_ref as usize,
            key_ref as usize,
            range,
        )
    };

    let read_stop = time::precise_time_s();
    let avg_time = (read_stop - read_start) / (num_reads as f64);

    println!(
        "[+] Size: {}, Values per key: {}, range: {}, times (s): {:?}",
        n_keys, vals_per_key, range, avg_time
    );

    result
    // *****
    // *****
    // *****
}

pub fn zerotrace(enclave: &SgxEnclave, n_keys: usize) -> sgx_status_t {

    pretty_env_logger::init().unwrap();
    let (mut client, mut server): (PathDOramClient<U8>, LocalServer<PathDOramClient<U8>>) =
        setup_oram(n_keys as u64);
    println!("After server setup!");

    let mut keys_and_positions = vec![];

    let num_reads: usize = 1000;
    for _ in 0..num_reads {
        let random_key = OramKey::rand() % (n_keys as u64);
        let pos = client.position_for_key(random_key);

        assert!(pos != NoPos);
        keys_and_positions.push((random_key, pos));
    }

    client.pos_map = PositionMap::new(0);

    // *****
    // *****
    // *****
    // Part inside here should be executed in the enclave.
    let client_ref = &client as *const PathDOramClient<_, _, _> as u64;
    let server_ref = &mut server as *mut LocalServer<PathDOramClient<U8>> as u64;
    let key_and_pos_ref = keys_and_positions.as_slice().as_ptr() as u64;
    let key_and_pos_len = keys_and_positions.len();

    let read_start = time::precise_time_s();
    let mut result = sgx_status_t::SGX_SUCCESS;
    let result = unsafe { oram_zerotrace(
        enclave.geteid(),
        &mut result,
        client_ref as usize,
        server_ref as usize,
        key_and_pos_ref as usize,
        key_and_pos_len,
    ) };

    let read_stop = time::precise_time_s();
    let avg_time = (read_stop - read_start) / num_reads as f64;

    println!(
        "\nSize: {}, time (s): {:?}",
        n_keys, avg_time
    );
    result
    // *****
    // *****
    // *****
}

pub fn doram(enclave: &SgxEnclave, n_keys: usize, block_size: u64) -> sgx_status_t {

    pretty_env_logger::init().unwrap();
    const NUM_READS: u64  = 1000;

    fn run_in_enclave(
        enclave: &SgxEnclave,
        client_ref: u64,
        server_ref: u64,
        key_and_pos_ref: u64,
        key_and_pos_len: usize,
        n_keys: usize,
        block_size: u64
    ) -> sgx_status_t {
        // *****
        // *****
        // *****
        // Part inside here should be executed in the enclave.
        let read_start = time::precise_time_s();

        let ret = unsafe { 
            oram_access(
                enclave.geteid(),
                &mut sgx_status_t::SGX_SUCCESS,
                client_ref as usize,
                server_ref as usize,
                key_and_pos_ref as usize,
                key_and_pos_len,
                block_size as usize,
            )
        };
        let read_stop = time::precise_time_s();
        let avg_time = (read_stop - read_start) / NUM_READS as f64;

        println!(
            "\nItems: {}, Blocksize: {}, time (s): {:?}",
            n_keys, block_size, avg_time
        );
        // *****
        // *****
        // *****
        ret
    }

    macro_rules! run_experiment_for_block_size {
        ($n:expr, $type_n:ty) => {
            {
                let (mut client, mut server): (PathDOramClient<$type_n>, LocalServer<PathDOramClient<$type_n>>) =
                                               setup_oram(n_keys as u64);
                println!("After server setup!");
                let mut keys_and_positions = vec![];
                for _ in 0..NUM_READS {
                    let random_key = OramKey::rand() % (n_keys as u64);
                    let pos = client.position_for_key(random_key);

                    assert!(pos != NoPos);
                    keys_and_positions.push((random_key, pos));
                }
                client.pos_map = PositionMap::new(0);
                let client_ref = &client as *const PathDOramClient<_, _, _> as u64;
                let server_ref = &mut server as *mut LocalServer<PathDOramClient<$type_n>> as u64;
                let key_and_pos_ref = &keys_and_positions as *const Vec<(OramKey, OramPos)> as u64;
                let key_and_pos_len = keys_and_positions.len();
                run_in_enclave(enclave, client_ref, server_ref, key_and_pos_ref, key_and_pos_len, n_keys, $n)
            }
        }
    }
    match block_size {
        8 => run_experiment_for_block_size!(8, U8),
        16 => run_experiment_for_block_size!(16, U16),
        32 => run_experiment_for_block_size!(32, U32),
        64 => run_experiment_for_block_size!(64, U64),
        128 => run_experiment_for_block_size!(128, U128),
        256 => run_experiment_for_block_size!(256, U256),
        512 => run_experiment_for_block_size!(512, U512),
        _   => panic!("Block size not supported, please input a block_size in the range {8, 16, 32, ..., 512}")
    }
}


fn setup_oram<N, C, M>(
    num_items: u64
) -> (
    PathDOramClient<N, C, M>,
    LocalServer<PathDOramClient<N, C, M>>,
)
    where
        N: ArrayLength<u8> + EncN,
        EncBlkSize<N>: ArrayLength<u8>,
        C: Encryptor,
        M: MerkleTree,
{
    let (mut client, _) = PathDOramClient::new(num_items, vec![]);
    let mut oram_data_map = Vec::with_capacity(num_items as usize);
    for i in 0..num_items {
        oram_data_map.push(
            (
                OramKey::new(i),
                BlockContent::with_slice(&[(i as u8 % 128u8); 8]),
            )
        );
    }
    println!("Before oram local setup to read");
    let server = client.local_setup(oram_data_map).unwrap();
    (client, server)
}
