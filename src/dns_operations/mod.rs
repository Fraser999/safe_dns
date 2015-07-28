// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

mod dns_configuration;

const DNS_TAG: u64 = 5; // TODO Get from routing

/// This is a representational structure for all maidsafe-dns operations
pub struct DnsOperations {
    client: ::std::sync::Arc<::std::sync::Mutex<::maidsafe_client::client::Client>>,
}

impl DnsOperations {
    /// Create a new instance of DnsOperations. It is intended that only one of this be created as
    /// it operates on global data such as files.
    pub fn new(client: ::std::sync::Arc<::std::sync::Mutex<::maidsafe_client::client::Client>>) -> Result<DnsOperations, ::errors::DnsError> {
        try!(dns_configuration::initialise_dns_configuaration(client.clone()));

        Ok(DnsOperations {
            client: client,
        })
    }

    /// Register one's own Dns - eg., pepsico.com, spandansharma.com, krishnakumar.in etc
    pub fn register_dns(&self,
                        long_name                      : String,
                        public_messaging_encryption_key: &::sodiumoxide::crypto::box_::PublicKey,
                        secret_messaging_encryption_key: &::sodiumoxide::crypto::box_::SecretKey,
                        services                       : &Vec<(String, (::routing::NameType, u64))>,
                        owners                         : Vec<::sodiumoxide::crypto::sign::PublicKey>,
                        private_signing_key            : &::sodiumoxide::crypto::sign::SecretKey,
                        data_encryption_keys           : Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                                 &::sodiumoxide::crypto::box_::SecretKey,
                                                                 &::sodiumoxide::crypto::box_::Nonce)>) -> Result<::maidsafe_client::client::StructuredData, ::errors::DnsError> {
        let mut saved_configs = try!(dns_configuration::get_dns_configuaration_data(self.client.clone()));
        if saved_configs.iter().any(|config| config.long_name == long_name) {
            Err(::errors::DnsError::DnsNameAlreadyRegistered)
        } else {
            let identifier = ::routing::NameType::new(::sodiumoxide::crypto::hash::sha512::hash(long_name.as_bytes()).0);

            let dns_record = Dns {
                long_name     : long_name.clone(),
                encryption_key: public_messaging_encryption_key.clone(),
                services      : services.iter().map(|a| a.clone()).collect(),
            };

            saved_configs.push(dns_configuration::DnsConfiguation {
                long_name         : long_name,
                encryption_keypair: (public_messaging_encryption_key.clone(),
                                     secret_messaging_encryption_key.clone())

            });
            try!(dns_configuration::write_dns_configuaration_data(self.client.clone(), &saved_configs));

            Ok(try!(::maidsafe_client::structured_data_operations::unversioned::create(self.client.clone(),
                                                                                       DNS_TAG,
                                                                                       identifier,
                                                                                       0,
                                                                                       try!(::maidsafe_client::utility::serialise(&dns_record)),
                                                                                       owners,
                                                                                       vec![],
                                                                                       private_signing_key,
                                                                                       data_encryption_keys)))
        }
    }

    /// Delete the Dns-Record
    pub fn delete_dns(&self,
                      long_name          : &String,
                      private_signing_key: &::sodiumoxide::crypto::sign::SecretKey) -> Result<::maidsafe_client::client::StructuredData, ::errors::DnsError> {
        let prev_struct_data = try!(self.get_housing_structured_data(long_name));

        let mut saved_configs = try!(dns_configuration::get_dns_configuaration_data(self.client.clone()));
        let pos = try!(saved_configs.iter().position(|config| config.long_name == *long_name).ok_or(::errors::DnsError::Unexpected("Programming Error - Investigate !!".to_string())));
        let _ = saved_configs.remove(pos);
        try!(dns_configuration::write_dns_configuaration_data(self.client.clone(), &saved_configs));

        Ok(try!(::maidsafe_client::structured_data_operations::unversioned::create(self.client.clone(),
                                                                                   DNS_TAG,
                                                                                   prev_struct_data.get_identifier().clone(),
                                                                                   prev_struct_data.get_version() + 1,
                                                                                   vec![],
                                                                                   prev_struct_data.get_owners().clone(),
                                                                                   prev_struct_data.get_previous_owners().clone(),
                                                                                   private_signing_key,
                                                                                   None)))
    }

    /// Get all the Dns-names registered by the user so far in the network.
    pub fn get_all_registered_names(&self) -> Result<Vec<String>, ::errors::DnsError> {
        Ok(try!(dns_configuration::get_dns_configuaration_data(self.client.clone())).iter().map(|a| a.long_name.clone()).collect())
    }

    /// Get the messaging encryption keys that the user has associated with one's particular Dns-name.
    pub fn get_messaging_encryption_keys(&self, long_name: &String) -> Result<(::sodiumoxide::crypto::box_::PublicKey,
                                                                               ::sodiumoxide::crypto::box_::SecretKey), ::errors::DnsError> {
        let dns_config_record = try!(self.find_dns_record(long_name));
        Ok(dns_config_record.encryption_keypair.clone())
    }

    /// Get all the services (www, blog, micro-blog etc) that user has associated with this
    /// Dns-name
    pub fn get_all_services(&self,
                            long_name           : &String,
                            data_decryption_keys: Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                          &::sodiumoxide::crypto::box_::SecretKey,
                                                          &::sodiumoxide::crypto::box_::Nonce)>) -> Result<Vec<String>, ::errors::DnsError> {
        let (_, dns_record) = try!(self.get_housing_sturctured_data_and_dns_record(long_name, data_decryption_keys));
        Ok(dns_record.services.keys().map(|a| a.clone()).collect())
    }

    /// Get the home directory (eg., homepage containing HOME.html, INDEX.html) for the given service.
    pub fn get_service_home_directory_key(&self,
                                          long_name           : &String,
                                          service_name        : &String,
                                          data_decryption_keys: Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                                        &::sodiumoxide::crypto::box_::SecretKey,
                                                                        &::sodiumoxide::crypto::box_::Nonce)>) -> Result<(::routing::NameType, u64), ::errors::DnsError> {
        let (_, dns_record) = try!(self.get_housing_sturctured_data_and_dns_record(long_name, data_decryption_keys));
        Ok(try!(dns_record.services.get(service_name).ok_or(::errors::DnsError::ServiceNotFound)).clone())
    }

    /// Add a new service for the given Dns-name.
    pub fn add_service(&self,
                       long_name                      : &String,
                       new_service                    : (String, (::routing::NameType, u64)),
                       private_signing_key            : &::sodiumoxide::crypto::sign::SecretKey,
                       data_encryption_decryption_keys: Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                                &::sodiumoxide::crypto::box_::SecretKey,
                                                                &::sodiumoxide::crypto::box_::Nonce)>) -> Result<::maidsafe_client::client::StructuredData, ::errors::DnsError> {
        Ok(try!(self.add_remove_service_impl(long_name, (new_service.0, Some(new_service.1)), private_signing_key, data_encryption_decryption_keys)))
    }

    /// Remove a service from the given Dns-name.
    pub fn remove_service(&self,
                          long_name                      : &String,
                          service_to_remove              : String,
                          private_signing_key            : &::sodiumoxide::crypto::sign::SecretKey,
                          data_encryption_decryption_keys: Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                                   &::sodiumoxide::crypto::box_::SecretKey,
                                                                   &::sodiumoxide::crypto::box_::Nonce)>) -> Result<::maidsafe_client::client::StructuredData, ::errors::DnsError> {
        Ok(try!(self.add_remove_service_impl(long_name, (service_to_remove, None), private_signing_key, data_encryption_decryption_keys)))
    }

    fn find_dns_record(&self, long_name: &String) -> Result<dns_configuration::DnsConfiguation, ::errors::DnsError> {
        let config_vec = try!(dns_configuration::get_dns_configuaration_data(self.client.clone()));
        Ok(try!(config_vec.iter().find(|config| config.long_name == *long_name).ok_or(::errors::DnsError::DnsRecordNotFound)).clone())
    }

    fn add_remove_service_impl(&self,
                               long_name                      : &String,
                               service                        : (String, Option<(::routing::NameType, u64)>),
                               private_signing_key            : &::sodiumoxide::crypto::sign::SecretKey,
                               data_encryption_decryption_keys: Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                                        &::sodiumoxide::crypto::box_::SecretKey,
                                                                        &::sodiumoxide::crypto::box_::Nonce)>) -> Result<::maidsafe_client::client::StructuredData, ::errors::DnsError> {
        let is_add_service = service.1.is_some();
        let (prev_struct_data, mut dns_record) = try!(self.get_housing_sturctured_data_and_dns_record(long_name,
                                                                                                      data_encryption_decryption_keys));

        if !is_add_service && !dns_record.services.contains_key(&service.0) {
            Err(::errors::DnsError::ServiceNotFound)
        } else if is_add_service && dns_record.services.contains_key(&service.0) {
            Err(::errors::DnsError::ServiceAlreadyExists)
        } else {
            if is_add_service {
                let _ = dns_record.services.insert(service.0, try!(service.1.ok_or(::errors::DnsError::Unexpected("Programming Error - Investigate !!".to_string()))));
            } else {
                let _ = dns_record.services.remove(&service.0);
            }

            Ok(try!(::maidsafe_client::structured_data_operations::unversioned::create(self.client.clone(),
                                                                                       DNS_TAG,
                                                                                       prev_struct_data.get_identifier().clone(),
                                                                                       prev_struct_data.get_version() + 1,
                                                                                       try!(::maidsafe_client::utility::serialise(&dns_record)),
                                                                                       prev_struct_data.get_owners().clone(),
                                                                                       prev_struct_data.get_previous_owners().clone(),
                                                                                       private_signing_key,
                                                                                       data_encryption_decryption_keys)))
        }
    }

    fn get_housing_sturctured_data_and_dns_record(&self,
                                                  long_name           : &String,
                                                  data_decryption_keys: Option<(&::sodiumoxide::crypto::box_::PublicKey,
                                                                                &::sodiumoxide::crypto::box_::SecretKey,
                                                                                &::sodiumoxide::crypto::box_::Nonce)>) -> Result<(::maidsafe_client::client::StructuredData,
                                                                                                                                  Dns), ::errors::DnsError> {
        let struct_data = try!(self.get_housing_structured_data(long_name));
        let dns_record = try!(::maidsafe_client::utility::deserialise(&try!(::maidsafe_client::structured_data_operations::unversioned::get_data(self.client.clone(),
                                                                                                                                                 &struct_data,
                                                                                                                                                 data_decryption_keys))));
        Ok((struct_data, dns_record))
    }

    fn get_housing_structured_data(&self, long_name: &String) -> Result<::maidsafe_client::client::StructuredData, ::errors::DnsError> {
        let _ = try!(self.find_dns_record(long_name));

        let identifier = ::routing::NameType::new(::sodiumoxide::crypto::hash::sha512::hash(long_name.as_bytes()).0);
        let location = ::maidsafe_client::client::StructuredData::compute_name(DNS_TAG, &identifier);
        let mut response_getter = try!(self.client.lock().unwrap().get(location, ::maidsafe_client::client::DataRequest::StructuredData(DNS_TAG)));
        if let ::maidsafe_client::client::Data::StructuredData(struct_data) = try!(response_getter.get()) {
            Ok(struct_data)
        } else {
            Err(::errors::DnsError::ClientError(::maidsafe_client::errors::ClientError::ReceivedUnexpectedData))
        }
    }
}

#[derive(Clone)] // TODO , Debug, Eq, PartialEq, RustcEncodable, RustcDecodable)]
struct Dns {
    long_name     : String,
    encryption_key: ::sodiumoxide::crypto::box_::PublicKey,
    services      : ::std::collections::HashMap<String, (::routing::NameType, u64)>,
}

impl ::rustc_serialize::Encodable for Dns {
    fn encode<E: ::rustc_serialize::Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
        let encryption_key_vec = self.encryption_key.0.iter().map(|a| *a).collect::<Vec<u8>>();

        ::cbor::CborTagEncode::new(100_001, &(&self.long_name,
                                              encryption_key_vec,
                                              &self.services)).encode(e)
    }
}

impl ::rustc_serialize::Decodable for Dns {
    fn decode<D: ::rustc_serialize::Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let _ = try!(d.read_u64());

        let (long_name,
             encryption_key_vec,
             services):
            (String,
             Vec<u8>,
             ::std::collections::HashMap<String, (::routing::NameType, u64)>) = try!(::rustc_serialize::Decodable::decode(d));

        let mut encryption_key_arr = [0u8; ::sodiumoxide::crypto::box_::PUBLICKEYBYTES];

        for it in encryption_key_vec.iter().enumerate() {
            encryption_key_arr[it.0] = *it.1;
        }

        Ok(Dns {
            long_name     : long_name,
            encryption_key: ::sodiumoxide::crypto::box_::PublicKey(encryption_key_arr),
            services      : services,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn register_and_delete_dns() {
        let client = ::std::sync::Arc::new(::std::sync::Mutex::new(eval_result!(::maidsafe_client::utility::test_utils::get_client())));
        let dns_operations = eval_result!(DnsOperations::new(client.clone()));

        let dns_name = eval_result!(::maidsafe_client::utility::generate_random_string(10));
        let messaging_keypair = ::sodiumoxide::crypto::box_::gen_keypair();
        let owners = vec![client.lock().unwrap().get_public_signing_key().clone()];

        let secret_signing_key = client.lock().unwrap().get_secret_signing_key().clone();

        // Register
        let mut struct_data = eval_result!(dns_operations.register_dns(dns_name.clone(),
                                                                       &messaging_keypair.0,
                                                                       &messaging_keypair.1,
                                                                       &vec![],
                                                                       owners.clone(),
                                                                       &secret_signing_key,
                                                                       None));

        eval_result!(client.lock().unwrap().put(struct_data.name().clone(), ::maidsafe_client::client::Data::StructuredData(struct_data)));

        // Get Services
        let services = eval_result!(dns_operations.get_all_services(&dns_name, None));
        assert_eq!(services.len(), 0);

        // Re-registering is not allowed
        match dns_operations.register_dns(dns_name.clone(),
                                          &messaging_keypair.0,
                                          &messaging_keypair.1,
                                          &vec![],
                                          owners.clone(),
                                          &secret_signing_key,
                                          None) {
            Ok(_) => panic!("Should have been an error"),
            Err(::errors::DnsError::DnsNameAlreadyRegistered) => (),
            Err(error) => panic!("{:?}", error),
        }

        // Delete
        struct_data = eval_result!(dns_operations.delete_dns(&dns_name, &secret_signing_key));
        eval_result!(client.lock().unwrap().delete(struct_data.name().clone(), ::maidsafe_client::client::Data::StructuredData(struct_data)));

        // Registering again should be allowed
        let _ = eval_result!(dns_operations.register_dns(dns_name,
                                                         &messaging_keypair.0,
                                                         &messaging_keypair.1,
                                                         &vec![],
                                                         owners,
                                                         &secret_signing_key,
                                                         None));
    }

    #[test]
    fn manipulate_services() {
        let client = ::std::sync::Arc::new(::std::sync::Mutex::new(eval_result!(::maidsafe_client::utility::test_utils::get_client())));
        let dns_operations = eval_result!(DnsOperations::new(client.clone()));

        let dns_name = eval_result!(::maidsafe_client::utility::generate_random_string(10));
        let messaging_keypair = ::sodiumoxide::crypto::box_::gen_keypair();

        let mut services = vec![("www".to_string(),     (::routing::NameType::new([123; 64]), 15000)),
                                ("blog".to_string(),    (::routing::NameType::new([124; 64]), 15000)),
                                ("bad-ass".to_string(), (::routing::NameType::new([124; 64]), 15000))];

        let owners = vec![client.lock().unwrap().get_public_signing_key().clone()];

        let secret_signing_key = client.lock().unwrap().get_secret_signing_key().clone();

        // Register
        let mut struct_data = eval_result!(dns_operations.register_dns(dns_name.clone(),
                                                                       &messaging_keypair.0,
                                                                       &messaging_keypair.1,
                                                                       &services,
                                                                       owners.clone(),
                                                                       &secret_signing_key,
                                                                       None));

        eval_result!(client.lock().unwrap().put(struct_data.name().clone(), ::maidsafe_client::client::Data::StructuredData(struct_data.clone())));

        // Get all dns-names
        let dns_records_vec = eval_result!(dns_operations.get_all_registered_names());
        assert_eq!(dns_records_vec.len(), 1);

        // Get all services for a dns-name
        let services_vec = eval_result!(dns_operations.get_all_services(&dns_name, None));
        assert_eq!(services.len(), services_vec.len());
        assert!(services.iter().all(|&(ref a, _)| services_vec.iter().find(|b| *a == **b).is_some()));

        match dns_operations.get_service_home_directory_key(&"bogus".to_string(), &services[0].0, None) {
            Ok(_) => panic!("Should have been an error"),
            Err(::errors::DnsError::DnsRecordNotFound) => (),
            Err(error) => panic!("{:?}", error),
        }

        // Get information about a service - the home-directory and its type
        let home_dir_key = eval_result!(dns_operations.get_service_home_directory_key(&dns_name, &services[1].0, None));
        assert_eq!(home_dir_key, services[1].1);

        // Remove a service
        let removed_service = services.remove(1);
        struct_data = eval_result!(dns_operations.remove_service(&dns_name, removed_service.0.clone(), &secret_signing_key, None));
        eval_result!(client.lock().unwrap().post(struct_data.name().clone(), ::maidsafe_client::client::Data::StructuredData(struct_data.clone())));

        // Get all services
        let services_vec = eval_result!(dns_operations.get_all_services(&dns_name, None));
        assert_eq!(services.len(), services_vec.len());
        assert!(services.iter().all(|&(ref a, _)| services_vec.iter().find(|b| *a == **b).is_some()));

        // Try to enquire about a deleted service
        match dns_operations.get_service_home_directory_key(&dns_name, &removed_service.0, None) {
            Ok(_) => panic!("Should have been an error"),
            Err(::errors::DnsError::ServiceNotFound) => (),
            Err(error) => panic!("{:?}", error),
        }

        // Add a service
        services.push(("added-service".to_string(), (::routing::NameType::new([126; 64]), 15000)));
        let services_size = services.len();
        struct_data = eval_result!(dns_operations.add_service(&dns_name, services[services_size - 1].clone(), &secret_signing_key, None));
        eval_result!(client.lock().unwrap().post(struct_data.name().clone(), ::maidsafe_client::client::Data::StructuredData(struct_data.clone())));

        // Get all services
        let services_vec = eval_result!(dns_operations.get_all_services(&dns_name, None));
        assert_eq!(services.len(), services_vec.len());
        assert!(services.iter().all(|&(ref a, _)| services_vec.iter().find(|b| *a == **b).is_some()));
    }
}