use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
use aws_sdk_s3::Client as S3Client;
use aws_types::region::Region;
use std::time::Duration;
//use std::{fs::File, io::Write};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use aws_sdk_s3::config::Credentials;

pub struct Client {
    pub client: S3Client,
    pub bucket_name: String,
}

impl Client {
    pub async fn new(
        endpoint: String,
        bucket_name: String,
        access_key: String,
        secret_key: String,
    ) -> Result<Self, aws_sdk_s3::Error> {
        let cred = Credentials::new(access_key, secret_key, None, None, "minio");
        let shared_config = aws_sdk_s3::config::Builder::new()
            .endpoint_url(endpoint)
            .credentials_provider(cred)
            .region(Region::new("eu-central-1"))
            .force_path_style(true) // apply bucketname as path param instead of pre-domain
            .build();

        let client = S3Client::from_conf(shared_config);

        Ok(Self {
            client,
            bucket_name,
        })
    }

    pub async fn fget_object(
        &self,
        object_name: &str,
        file_path: &str,
    ) -> Result<usize, anyhow::Error> {
        let mut file = File::create(file_path).await?;

        let mut object = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(object_name)
            .send()
            .await?;

        let mut byte_count = 0_usize;
        while let Some(bytes) = object.body.try_next().await? {
            let bytes_len = bytes.len();
            file.write_all(&bytes).await?;
            byte_count += bytes_len;
        }
        //file.flush().await?;

        Ok(byte_count)
    }

    pub async fn get_object(&self, object_name: &str) -> Result<Vec<u8>, anyhow::Error> {
        let mut ret: Vec<u8> = vec![];
        let mut object = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(object_name)
            .send()
            .await?;

        while let Some(bytes) = object.body.try_next().await? {
            ret.write_all(&bytes).await?;
        }

        Ok(ret)
    }

    // Adds an object to a bucket and returns a public URI.
    pub async fn get_signed_uri(
        self,
        bucket: &str,
        object: &str,
        expires_in: u64,
    ) -> Result<String, anyhow::Error> {
        let expires_in = Duration::from_secs(expires_in);

        let presigned_request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(object)
            .presigned(PresigningConfig::expires_in(expires_in)?)
            .await?;

        Ok(presigned_request.uri().to_string())
    }

    pub async fn fput_object(
        &self,
        object_name: &str,
        file_path: &str,
    ) -> Result<(), anyhow::Error> {
        let body = ByteStream::read_from()
            .path(file_path)
            // Artificially limit the buffer size to ensure the file has multiple
            // progress steps.
            //.buffer_size(2048)
            .build()
            .await?;

        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(object_name)
            .body(body)
            .send()
            .await?;

        Ok(())
    }

    //pub async fn put_object(
    //    &self,
    //    object_name: &str,
    //    data: Vec<u8>,
    //) -> Result<(), aws_sdk_s3::Error> {
    //    let stream = ByteStream::from(data);

    //    let result = self
    //        .client
    //        .put_object()
    //        .bucket(&self.bucket_name)
    //        .key(object_name)
    //        .body(stream)
    //        .send()
    //        .await?;

    //    println!(
    //        "Uploaded {} bytes",
    //        result.content_length.unwrap_or_default()
    //    );
    //    Ok(())
    //}

    pub async fn list_objects(
        &self,
        object_prefix: &str,
    ) -> Result<Vec<String>, aws_sdk_s3::Error> {
        let mut objects = Vec::new();
        let mut resp = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .prefix(object_prefix)
            .send()
            .await?;

        resp.contents
            .as_mut()
            .unwrap()
            .iter_mut()
            .for_each(|object| {
                objects.push(object.key.take().unwrap());
            });

        //paginator?
        //let mut paginator = request.send().await?;
        //while let Some(page) = paginator.next_page().await? {
        //    for object in page.contents.unwrap_or_default() {
        //        if let Some(key) = object.key {
        //            objects.push(key);
        //        }
        //    }
        //}

        Ok(objects)
    }

    pub async fn delete_object(&self, object_name: &str) -> Result<(), aws_sdk_s3::Error> {
        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(object_name)
            .send()
            .await?;
        Ok(())
    }

    pub async fn delete_objects(&self, objects: Vec<String>) -> Result<(), aws_sdk_s3::Error> {
        let mut delete_objects: Vec<ObjectIdentifier> = vec![];

        for obj in objects {
            let obj_id = ObjectIdentifier::builder()
                .set_key(Some(obj))
                .build()
                .expect("building ObjectIdentifier");
            delete_objects.push(obj_id);
        }

        let delete = Delete::builder()
            .set_objects(Some(delete_objects))
            .build()
            .expect("building Delete");

        self.client
            .delete_objects()
            .bucket(&self.bucket_name)
            .delete(delete)
            .send()
            .await?;

        println!("Objects deleted.");

        Ok(())
    }
}
