+++
description = 'Review Spring Data JPA where the project uses it for persistence.'
uses = [{ scope = "local", resource_id = "spring" }]
official_sources = [{ label = 'Spring Data JPA reference; resolve the consuming version', url = 'https://docs.spring.io/spring-data/jpa/reference/' }]

[guide]

[stack]
extends = [{ scope = "local", resource_id = "spring" }]
role_uses = [{ scope = "local", resource_id = "spring-data-jpa" }]
+++

# Check JPA queries transactions and managed state

Apply this guidance when Spring Data JPA is selected, after Spring, Java and the common method. Identify the library and provider versions, repository bootstrap mode, entity model, persistence unit and transaction manager. Check query validation and actual persistence through their respective running paths.

## Follow the query to its result

Use a derived method when it expresses the lookup clearly. Choose declared JPQL, native SQL, projections or a custom implementation when they better express the required result and access path. Trace properties, parameters, null handling, ordering, result counts and the mapped schema.

Spring Data parses derived properties at runtime. Eager, lazy and deferred repository initialization change when that check occurs. Test repository creation, execution against the selected database and the caller's actual result separately where needed. Check provider-specific query extensions against that provider.

## Track transactions and already loaded entities

Inherited CRUD methods and declared query methods have different transaction defaults. Follow the whole unit of work and any outer transaction through the selected Spring interception path; the outer transaction can determine the effective settings. `@Modifying` changes how a declared `@Query` executes. Establish its transaction and `readOnly` setting separately, and check writes explicitly rather than relying on a read-only hint as a universal guard.

A bulk change can leave managed entities stale. Choose flushing, clearing, reloading and affected-row checks according to the required result and pending changes. In the examined API, `flushAutomatically` and `clearAutomatically` are separate controls, both false by default. Clearing can discard unflushed work. Follow later reads and committed data to check whether returned objects still represent the stored state.

Derived deletion can load and remove entities with lifecycle callbacks; a bulk query bypasses that per-entity route. Preserve required callbacks, cascades and audit behaviour when replacing one with the other, and compare memory and execution costs at the intended scale. Keep API representations and persistence graphs separate as the owner requires.

## Establish what a test database proves

Identify who creates the schema, how seed data is loaded and how long the fixture lives. An ORM-created schema can exercise entity queries; production migrations, grants, triggers and extensions need their own checks. Give reused databases and cached application contexts compatible state and cleanup rules.

Spring test-managed rollback covers its participating transaction and thread. Real HTTP-server work, asynchronous work, preemptive test timeouts and independent transactions can write outside it. Trace which writes participate and isolate or clean the remaining effects.

Use native checks for the changed query, transaction, managed state or lifecycle. Reuse current applicable results and follow the repository's test-writing rules. Entity conventions, schema and migration authority, exact commands and production effects remain with the repository and its owner.

## Read after a bulk update

A bulk `@Modifying` query changes status while the entity is already managed. Read before and after the change in the same transaction, then flush or clear as the intended behaviour requires. Check whether stale managed state could overwrite the update.

## Versioned references

- [Spring Data JPA 3.5 query creation and modifying queries](https://docs.spring.io/spring-data/jpa/reference/3.5/jpa/query-methods.html)
- [Spring Data JPA 3.5 transaction boundaries](https://docs.spring.io/spring-data/jpa/reference/3.5/jpa/transactions.html)
- [Spring Data JPA 3.5 repository bootstrap](https://docs.spring.io/spring-data/jpa/reference/3.5/repositories/create-instances.html)
- [Spring Data JPA 3.5.0 modifying-query controls](https://docs.spring.io/spring-data/jpa/docs/3.5.0/api/org/springframework/data/jpa/repository/Modifying.html)
- [Spring Framework 6.2 test-managed transactions](https://docs.spring.io/spring-framework/reference/6.2/testing/testcontext-framework/tx.html)
- [Spring Boot 3.5 test server and context boundaries](https://docs.spring.io/spring-boot/3.5/reference/testing/spring-boot-applications.html)

These describe the examined version family. Resolve the actual library, provider and database before relying on version-specific behaviour.
