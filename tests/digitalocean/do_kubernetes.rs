extern crate test_utilities;

use std::fs::File;
use std::io::Read;

use test_utilities::digitalocean::DO_KUBERNETES_VERSION;
use tracing::{error, span, Level};

use qovery_engine::cloud_provider::digitalocean::common::get_uuid_of_cluster_from_name;
use qovery_engine::cloud_provider::digitalocean::kubernetes::DOKS;
use qovery_engine::cmd::kubectl::{kubectl_exec_create_namespace, kubectl_exec_delete_namespace};
use qovery_engine::constants::DIGITAL_OCEAN_TOKEN;

use self::test_utilities::cloudflare::dns_provider_cloudflare;
use self::test_utilities::digitalocean::{digital_ocean_token, get_kube_cluster_name_from_uuid};
use self::test_utilities::utilities::{engine_run_test, generate_id};
use qovery_engine::cloud_provider::kubernetes::Kubernetes;

//#[test]
//#[ignore]
fn create_doks_cluster_in_fra_10() {
    engine_run_test(|| {
        let span = span!(Level::INFO, "test", name = "create_doks_cluster_in_fra_10");
        let _enter = span.enter();

        let cluster_id = "my-first-doks-10";
        let cluster_name = "do-kube-cluster-fra1-10";
        let region = "fra1";

        let context = test_utilities::utilities::context();

        let engine = test_utilities::digitalocean::docker_cr_do_engine(&context);
        let session = engine.session().unwrap();
        let mut tx = session.transaction();

        let digitalocean = test_utilities::digitalocean::cloud_provider_digitalocean(&context);
        let nodes = test_utilities::digitalocean::do_kubernetes_nodes();

        let cloudflare = dns_provider_cloudflare(&context);

        let mut file = File::open("tests/assets/do-options.json").unwrap();
        let mut read_buf = String::new();
        file.read_to_string(&mut read_buf).unwrap();

        let options_result =
            serde_json::from_str::<qovery_engine::cloud_provider::digitalocean::kubernetes::Options>(read_buf.as_str());

        let kubernetes = DOKS::new(
            context.clone(),
            cluster_id.clone(),
            cluster_name.clone(),
            DO_KUBERNETES_VERSION,
            region.clone(),
            &digitalocean,
            &cloudflare,
            options_result.expect("Oh my satan an error in test... Options options options"),
            nodes,
        );
        match tx.create_kubernetes(&kubernetes) {
            Err(err) => panic!("{:?}", err),
            _ => {}
        }
        tx.commit();

        // TESTING: Kube cluster UUID is OK ?
        let res_uuid = get_uuid_of_cluster_from_name(digital_ocean_token().as_str(), cluster_name.clone());
        match res_uuid {
            Ok(uuid) => assert_eq!(get_kube_cluster_name_from_uuid(uuid.as_str()), cluster_name.clone()),
            Err(e) => {
                error!("{:?}", e.message);
                assert!(false);
            }
        }

        //TESTING: Kubeconfig DOWNLOAD
        //TODO: Fix the kubernetes_config_path fn
        match kubernetes.config_file_path() {
            Ok(file) => {
                let do_credentials_envs = vec![(DIGITAL_OCEAN_TOKEN, digitalocean.token.as_str())];
                // testing kubeconfig file
                let namespace_to_test = generate_id();
                match kubectl_exec_create_namespace(
                    file.clone(),
                    namespace_to_test.clone().as_str(),
                    None,
                    do_credentials_envs.clone(),
                ) {
                    Ok(_) => {
                        // Delete created namespace
                        match kubectl_exec_delete_namespace(
                            file,
                            namespace_to_test.as_str(),
                            do_credentials_envs.clone(),
                        ) {
                            Ok(_) => assert!(true),
                            Err(_) => assert!(false),
                        }
                    }
                    Err(_) => assert!(false),
                }
            }
            Err(_) => assert!(false),
        }
        return "create_doks_cluster_in_fra_10".to_string();
    })
}
