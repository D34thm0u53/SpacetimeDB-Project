---
applyTo: '**'
---
Coding standards, domain knowledge, and preferences that AI should follow.Row Level Security (RLS)

Row Level Security (RLS) allows module authors to restrict which rows of a public table each client can access. These access rules are expressed in SQL and evaluated automatically for queries and subscriptions.
Enabling RLS

RLS is currently experimental and must be explicitly enabled in your module.

To enable RLS, activate the unstable feature in your project's Cargo.toml:

spacetimedb = { version = "...", features = ["unstable"] }
 

How It Works

RLS rules are expressed in SQL and declared as constants of type Filter.

use spacetimedb::{client_visibility_filter, Filter};

/// A client can only see their account
#[client_visibility_filter]
const ACCOUNT_FILTER: Filter = Filter::Sql(
    "SELECT * FROM account WHERE identity = :sender"
);
 

A module will fail to publish if any of its RLS rules are invalid or malformed.
`:sender`

You can use the special :sender parameter in your rules for user specific access control. This parameter is automatically bound to the requesting client's Identity.

Note that module owners have unrestricted access to all tables regardless of RLS.
Semantic Constraints

RLS rules are similar to subscriptions in that logically they act as filters on a particular table. Also like subscriptions, arbitrary column projections are not allowed. Joins are allowed, but each rule must return rows from one and only one table.
Multiple Rules Per Table

Multiple rules may be declared for the same table and will be evaluated as a logical OR. This means clients will be able to see to any row that matches at least one of the rules.
Example

use spacetimedb::{client_visibility_filter, Filter};

/// A client can only see their account
#[client_visibility_filter]
const ACCOUNT_FILTER: Filter = Filter::Sql(
    "SELECT * FROM account WHERE identity = :sender"
);

/// An admin can see all accounts
#[client_visibility_filter]
const ACCOUNT_FILTER_FOR_ADMINS: Filter = Filter::Sql(
    "SELECT account.* FROM account JOIN admin WHERE admin.identity = :sender"
);
 

Recursive Application

RLS rules can reference other tables with RLS rules, and they will be applied recursively. This ensures that data is never leaked through indirect access patterns.
Example

use spacetimedb::{client_visibility_filter, Filter};

/// A client can only see their account
#[client_visibility_filter]
const ACCOUNT_FILTER: Filter = Filter::Sql(
    "SELECT * FROM account WHERE identity = :sender"
);

/// An admin can see all accounts
#[client_visibility_filter]
const ACCOUNT_FILTER_FOR_ADMINS: Filter = Filter::Sql(
    "SELECT account.* FROM account JOIN admin WHERE admin.identity = :sender"
);

/// Explicitly filtering by client identity in this rule is not necessary,
/// since the above RLS rules on `account` will be applied automatically.
/// Hence a client can only see their player, but an admin can see all players.
#[client_visibility_filter]
const PLAYER_FILTER: Filter = Filter::Sql(
    "SELECT p.* FROM account a JOIN player p ON a.id = p.id"
);
 

And while self-joins are allowed, in general RLS rules cannot be self-referential, as this would result in infinite recursion.
Example: Self-Join

use spacetimedb::{client_visibility_filter, Filter};

/// A client can only see players on their same level
#[client_visibility_filter]
const PLAYER_FILTER: Filter = Filter::Sql("
    SELECT q.*
    FROM account a
    JOIN player p ON a.id = p.id
    JOIN player q on p.level = q.level
    WHERE a.identity = :sender
");
 

Example: Recursive Rules

This module will fail to publish because each rule depends on the other one.

use spacetimedb::{client_visibility_filter, Filter};

/// An account must have a corresponding player
#[client_visibility_filter]
const ACCOUNT_FILTER: Filter = Filter::Sql(
    "SELECT a.* FROM account a JOIN player p ON a.id = p.id WHERE a.identity = :sender"
);

/// A player must have a corresponding account
#[client_visibility_filter]
const PLAYER_FILTER: Filter = Filter::Sql(
    "SELECT p.* FROM account a JOIN player p ON a.id = p.id WHERE a.identity = :sender"
);
 

Usage in Subscriptions

RLS rules automatically apply to subscriptions so that if a client subscribes to a table with RLS filters, the subscription will only return rows that the client is allowed to see.

While the contraints and limitations outlined in the reference docs do not apply to RLS rules, they do apply to the subscriptions that use them. For example, it is valid for an RLS rule to have more joins than are supported by subscriptions. However a client will not be able to subscribe to the table for which that rule is defined.
Best Practices

    Use :sender for client specific filtering.
    Follow the SQL best practices for optimizing your RLS rules.
