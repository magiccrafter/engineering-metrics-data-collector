/// Defines the store for the application.
///
/// The store is responsible for managing the database connection pool and providing access to the database.
pub mod store;

/// Defines the clients for the 3rd party services.
///
/// The clients contain the logic for interacting with the 3rd party HTTP services (GraphQL & RESTFul).
pub mod client;

/// Defines the components for the application.
///
/// The components contain the logic for interacting with the database and the 3rd party services.
pub mod component;
