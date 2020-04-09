use jammdb::{Data, Error, DB};
use rand::prelude::*;
use std::collections::HashSet;

mod common;

#[test]
fn small_deletes() {
	for _ in 0..10 {
		test_deletes(100).unwrap();
	}
}

#[test]
fn medium_deletes() {
	for _ in 0..10 {
		test_deletes(1000).unwrap();
	}
}

#[test]
fn large_deletes() {
	for _ in 0..10 {
		test_deletes(1000).unwrap();
	}
}

fn test_deletes(highest_int: u64) -> Result<(), Error> {
	let random_file = common::RandomFile::new();
	let mut deleted = HashSet::new();
	let mut rng = rand::thread_rng();
	{
		let mut db = DB::open(&random_file.path)?;
		{
			let mut tx = db.tx(true)?;
			let b = tx.create_bucket("abc")?;
			for i in 0..highest_int {
				b.put(i.to_be_bytes(), i.to_string())?;
			}
			tx.commit()?;
		}
		let mut ids_to_delete: Vec<u64> = (0..highest_int).collect();
		ids_to_delete.shuffle(&mut rng);
		let mut id_iter = ids_to_delete.iter();

		loop {
			{
				let mut tx = db.tx(true)?;
				let b = tx.get_bucket("abc")?;
				// delete between 0 and 100 random items
				for _ in 0..rng.gen_range(10, 100) {
					let i = id_iter.next();
					if i.is_none() {
						break;
					}
					let i = i.unwrap();
					if deleted.insert(i) {
						let kv = b.delete(i.to_be_bytes())?;
						assert_eq!(kv.key(), i.to_be_bytes());
						assert_eq!(kv.value(), i.to_string().as_bytes());
					}
				}
				for i in 0..highest_int {
					let data = b.get(i.to_be_bytes());
					if deleted.contains(&i) {
						assert_eq!(data, None)
					} else {
						match data {
							Some(Data::KeyValue(kv)) => {
								assert_eq!(kv.key(), i.to_be_bytes());
								assert_eq!(kv.value(), i.to_string().as_bytes());
							}
							_ => panic!("Expected Data::KeyValue"),
						}
					}
				}
				tx.commit()?;
			}
			{
				let mut tx = db.tx(true)?;
				let b = tx.get_bucket("abc")?;
				for i in 0..highest_int {
					let data = b.get(i.to_be_bytes());
					if deleted.contains(&i) {
						assert_eq!(data, None)
					} else {
						match data {
							Some(Data::KeyValue(kv)) => {
								assert_eq!(kv.key(), i.to_be_bytes());
								assert_eq!(kv.value(), i.to_string().as_bytes());
							}
							_ => panic!("Expected Some(Data::KeyValue) at index {}: {:?}", i, data),
						}
					}
				}
			}
			if deleted.len() == ids_to_delete.len() {
				break;
			}
		}
		db.check()
	}
}

#[test]
fn delete_simple_bucket() -> Result<(), Error> {
	let random_file = common::RandomFile::new();
	let mut db = DB::open(&random_file.path)?;
	{
		let mut tx = db.tx(true)?;
		let b = tx.create_bucket("abc")?;
		for i in 0..10_u64 {
			b.put(i.to_be_bytes(), i.to_string())?;
		}
		tx.commit()?;
	}
	{
		let mut tx = db.tx(true)?;
		tx.delete_bucket("abc")?;
		assert_eq!(tx.get_bucket("abc").err(), Some(Error::BucketMissing));
		// delete a freshly created bucket
		let b = tx.create_bucket("def")?;
		b.put("some", "data")?;
		tx.delete_bucket("def")?;

		tx.commit()?;
	}
	{
		let mut tx = db.tx(false)?;
		assert_eq!(tx.get_bucket("abc").err(), Some(Error::BucketMissing));
		assert_eq!(tx.get_bucket("def").err(), Some(Error::BucketMissing));
	}
	db.check()
}

#[test]
fn delete_large_bucket_with_large_nested_buckets() -> Result<(), Error> {
	let random_file = common::RandomFile::new();
	let mut db = DB::open(&random_file.path)?;
	{
		let mut tx = db.tx(true)?;
		let b = tx.create_bucket("abc")?;
		for i in 0..50_u64 {
			let sub_bucket = b.create_bucket(i.to_be_bytes())?;
			for i in 0..1000_u64 {
				sub_bucket.put(i.to_be_bytes(), i.to_string().repeat(10))?;
			}
		}
		tx.commit()?;
	}
	{
		let mut tx = db.tx(true)?;
		tx.delete_bucket("abc")?;
		assert_eq!(tx.get_bucket("abc").err(), Some(Error::BucketMissing));
		tx.commit()?;
	}
	{
		let mut tx = db.tx(false)?;
		assert_eq!(tx.get_bucket("abc").err(), Some(Error::BucketMissing));
	}
	db.check()
}
