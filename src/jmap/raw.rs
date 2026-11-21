//! # Raw JMAP calls
//!
//! `Foo/query` chained into `Foo/get`, `Foo/get` alone and a one-object
//! `Foo/set`, each answer kept as sent: protocol commands print the server's
//! payload rather than re-serialize it (pimalaya-cli output.md,
//! *Protocol commands are raw-faithful*).

use anyhow::{Context, Result};
use io_jmap::{
    client::JmapClientStd,
    rfc8620::{JMAP_CORE_CAPABILITY, error::JmapMethodError, request::JmapRequest},
    rfc8621::{
        JMAP_MAIL_CAPABILITY, email_submission::JMAP_SUBMISSION_CAPABILITY,
        vacation_response::JMAP_VACATION_RESPONSE_CAPABILITY,
    },
};
use serde::Serialize;
use serde_json::{Map, Value, json};

/// One answer of a raw listing, as the server sent it.
#[derive(Clone, Debug, Default)]
pub struct JmapRawList {
    /// The objects of the `Foo/get` `list`.
    pub list: Vec<Value>,
    /// The ids the `Foo/get` did not find (`notFound`).
    pub not_found: Vec<String>,
    /// The other keys of both answers, the `ids` and `accountId` they
    /// repeat left out.
    pub rest: Map<String, Value>,
}

/// Runs `<object>/query` with `args`, its ids chained into `<object>/get`.
pub fn query(
    client: &mut JmapClientStd,
    object: &str,
    mut args: Map<String, Value>,
) -> Result<JmapRawList> {
    let account = account_id(client)?;
    args.insert(String::from("accountId"), json!(account));

    let query = format!("{object}/query");
    let get = json!({
        "accountId": account,
        "#ids": {"resultOf": "q", "name": query, "path": "/ids"},
    });
    let calls = vec![
        (query, Value::Object(args), String::from("q")),
        (format!("{object}/get"), get, String::from("g")),
    ];

    let mut answers = send(client, object, calls)?.into_iter();
    let query = answers.next().unwrap_or_default();
    let get = answers.next().unwrap_or_default();

    let mut list = listed(get);
    for (key, value) in query {
        if key != "ids" && key != "accountId" {
            list.rest.insert(key, value);
        }
    }

    Ok(list)
}

/// Runs `<object>/get` on `ids`, on every object of its type when `None`.
pub fn get(
    client: &mut JmapClientStd,
    object: &str,
    ids: Option<&[String]>,
) -> Result<JmapRawList> {
    let account = account_id(client)?;
    let args = json!({"accountId": account, "ids": ids});
    let calls = vec![(format!("{object}/get"), args, String::from("g"))];

    let get = send(client, object, calls)?.into_iter().next();
    Ok(listed(get.unwrap_or_default()))
}

/// The answer of a `Foo/set` creating one object, as the server sent it.
#[derive(Clone, Debug, Default)]
pub struct JmapRawCreated {
    /// The object created, the properties the server set (`id` among
    /// them), `None` when it was not created.
    pub created: Option<Value>,
    /// Why the object was not created (`notCreated`), as the server sent
    /// it.
    pub not_created: Option<Value>,
    /// The other keys of the answer, `accountId` and the maps left out:
    /// `oldState` and `newState`.
    pub rest: Map<String, Value>,
}

/// Runs `<object>/set` creating `create` under the creation id `key`.
pub fn create(
    client: &mut JmapClientStd,
    object: &str,
    key: &str,
    create: Value,
) -> Result<JmapRawCreated> {
    let account = account_id(client)?;
    let args = json!({"accountId": account, "create": {key: create}});
    let calls = vec![(format!("{object}/set"), args, String::from("s"))];

    let mut set = send(client, object, calls)?
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut take = |name: &str| match set.remove(name) {
        Some(Value::Object(mut map)) => map.remove(key),
        _ => None,
    };
    let created = take("created");
    let not_created = take("notCreated");

    set.remove("accountId");
    for name in ["updated", "destroyed", "notUpdated", "notDestroyed"] {
        set.remove(name);
    }

    Ok(JmapRawCreated {
        created,
        not_created,
        rest: set,
    })
}

/// Splits a `Foo/get` answer into its list, its ids not found and the
/// rest.
fn listed(mut get: Map<String, Value>) -> JmapRawList {
    let list = match get.remove("list") {
        Some(Value::Array(list)) => list,
        _ => Vec::new(),
    };
    let not_found = match get.remove("notFound") {
        Some(Value::Array(ids)) => ids
            .into_iter()
            .filter_map(|id| id.as_str().map(String::from))
            .collect(),
        _ => Vec::new(),
    };
    get.remove("accountId");

    JmapRawList {
        list,
        not_found,
        rest: get,
    }
}

/// The account the mail capability names in the session.
fn account_id(client: &JmapClientStd) -> Result<String> {
    let session = client.session().context("No JMAP session")?;
    let account = session.account_for(JMAP_MAIL_CAPABILITY)?;
    Ok(String::from(account))
}

/// Sends `calls` as one request and returns each answer's arguments, in
/// order; a method answering `error` fails the whole listing with it.
fn send(
    client: &mut JmapClientStd,
    object: &str,
    calls: Vec<(String, Value, String)>,
) -> Result<Vec<Map<String, Value>>> {
    let mut using = vec![
        JMAP_CORE_CAPABILITY.to_string(),
        JMAP_MAIL_CAPABILITY.to_string(),
    ];
    if matches!(object, "EmailSubmission" | "Identity") {
        using.push(JMAP_SUBMISSION_CAPABILITY.to_string());
    }
    if object == "VacationResponse" {
        using.push(JMAP_VACATION_RESPONSE_CAPABILITY.to_string());
    }

    let request = JmapRequest {
        using,
        method_calls: calls,
        created_ids: None,
    };
    let response = client.send_raw(request)?;

    let mut answers = Vec::new();
    for (name, args, call_id) in response.method_responses {
        if name == "error" {
            let err: JmapMethodError = serde_json::from_str(args.get())
                .with_context(|| format!("Cannot read the error of JMAP call `{call_id}`"))?;
            return Err(err.into());
        }

        let args = serde_json::from_str(args.get())
            .with_context(|| format!("Cannot read the answer to JMAP call `{call_id}`"))?;
        answers.push(args);
    }

    Ok(answers)
}

/// The arguments of a `Foo/query` asking for its `total`: `filter` and
/// `sort` when given, and the window of `limit` ids from `position`.
pub fn query_args<F: Serialize, S: Serialize>(
    filter: Option<F>,
    sort: Option<S>,
    position: u64,
    limit: u64,
) -> Result<Map<String, Value>> {
    let mut args = Map::new();
    if let Some(filter) = filter {
        args.insert(String::from("filter"), serde_json::to_value(filter)?);
    }
    if let Some(sort) = sort {
        args.insert(String::from("sort"), serde_json::to_value(sort)?);
    }
    args.insert(String::from("position"), json!(position));
    args.insert(String::from("limit"), json!(limit));
    args.insert(String::from("calculateTotal"), json!(true));
    Ok(args)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use io_jmap::{client::JmapClientStd, rfc8620::session::JmapSession};
    use secrecy::SecretString;
    use serde_json::{Map, json};

    use crate::{
        http_script::{Script, ok},
        jmap::raw,
    };

    /// A client over a session naming `a1` as the mail account, its
    /// requests answered by `responses`.
    fn client(responses: Vec<serde_json::Value>) -> JmapClientStd {
        client_and_requests(responses).0
    }

    /// A client as [`client`] builds it, beside the requests it sends.
    fn client_and_requests(
        responses: Vec<serde_json::Value>,
    ) -> (JmapClientStd, Arc<Mutex<Vec<String>>>) {
        let session: JmapSession = serde_json::from_value(json!({
            "username": "u",
            "accounts": {},
            "primaryAccounts": {"urn:ietf:params:jmap:mail": "a1"},
            "capabilities": {},
            "apiUrl": "https://jmap.example.org/api/",
            "downloadUrl": "",
            "uploadUrl": "",
            "eventSourceUrl": "",
            "state": "s",
        }))
        .unwrap();
        let responses = responses
            .into_iter()
            .map(|body| ok("application/json", &body.to_string()))
            .collect();
        let (script, requests) = Script::new(responses);
        let auth = SecretString::from("token");
        let client = JmapClientStd::from_parts(script, auth, session);
        (client, requests)
    }

    /// The objects print as sent beside the query's keys, `notFound` kept
    /// apart for the item errors.
    #[test]
    fn a_query_keeps_what_the_server_sent() {
        let answer = json!({"methodResponses": [
            ["Email/query", {"accountId": "a1", "queryState": "q1", "canCalculateChanges": false, "position": 0, "ids": ["e1", "e2"], "total": 5}, "q"],
            ["Email/get", {"accountId": "a1", "state": "s1", "list": [{"id": "e1", "x-vendor": 1}], "notFound": ["e2"]}, "g"],
        ], "sessionState": "s"});
        let mut client = client(vec![answer]);

        let list = raw::query(&mut client, "Email", Map::new()).unwrap();

        assert_eq!(list.list, [json!({"id": "e1", "x-vendor": 1})]);
        assert_eq!(list.not_found, ["e2"]);
        let rest = json!({"queryState": "q1", "canCalculateChanges": false, "position": 0, "total": 5, "state": "s1"});
        assert_eq!(serde_json::Value::Object(list.rest), rest);
    }

    /// A vacation response is asked for under its own capability, and
    /// kept as sent.
    #[test]
    fn a_vacation_response_get_names_its_capability() {
        let vacation = json!({"id": "singleton", "isEnabled": false, "x-vendor": 1});
        let answer = json!({"methodResponses": [
            ["VacationResponse/get", {"accountId": "a1", "state": "s1", "list": [vacation], "notFound": []}, "g"],
        ], "sessionState": "s"});
        let (mut client, requests) = client_and_requests(vec![answer]);

        let list = raw::get(&mut client, "VacationResponse", None).unwrap();

        assert_eq!(list.list, [vacation]);
        let requests = requests.lock().unwrap();
        assert!(requests[0].contains("urn:ietf:params:jmap:vacationresponse"));
    }

    /// A method answering `error` fails the listing with that error.
    #[test]
    fn a_method_error_fails_the_listing() {
        let answer = json!({"methodResponses": [
            ["error", {"type": "forbidden"}, "g"],
        ], "sessionState": "s"});
        let mut client = client(vec![answer]);

        let err = raw::get(&mut client, "Email", Some(&[String::from("e1")])).unwrap_err();
        assert!(
            err.downcast_ref::<io_jmap::rfc8620::error::JmapMethodError>()
                .is_some()
        );
    }

    /// A get without ids sends `ids` null, an identity under the submission
    /// capability, and keeps each object as sent.
    #[test]
    fn a_get_without_ids_asks_for_every_object() {
        let answer = json!({"methodResponses": [
            ["Identity/get", {"accountId": "a1", "state": "s1", "list": [{"id": "i1", "email": "a@x.org", "x-vendor": 1}], "notFound": []}, "g"],
        ], "sessionState": "s"});
        let (mut client, requests) = client_and_requests(vec![answer]);

        let list = raw::get(&mut client, "Identity", None).unwrap();

        assert_eq!(
            list.list,
            [json!({"id": "i1", "email": "a@x.org", "x-vendor": 1})]
        );
        assert_eq!(serde_json::Value::Object(list.rest), json!({"state": "s1"}));
        let request = requests.lock().unwrap().join("");
        assert!(request.contains(r#""ids":null"#), "{request}");
        assert!(
            request.contains("urn:ietf:params:jmap:submission"),
            "{request}"
        );
    }

    /// A create keeps the object as the server set it and the set's states,
    /// or what the server answered for the object it did not create.
    #[test]
    fn a_create_keeps_what_the_server_set() {
        let created = json!({"methodResponses": [
            ["EmailSubmission/set", {"accountId": "a1", "oldState": "1", "newState": "2", "created": {"e1": {"id": "s1", "undoStatus": "pending", "x-vendor": 1}}}, "s"],
        ], "sessionState": "s"});
        let refused = json!({"methodResponses": [
            ["EmailSubmission/set", {"accountId": "a1", "newState": "2", "notCreated": {"e1": {"type": "forbiddenFrom"}}}, "s"],
        ], "sessionState": "s"});
        let mut client = client(vec![created, refused]);

        let set = raw::create(&mut client, "EmailSubmission", "e1", json!({})).unwrap();
        let object = json!({"id": "s1", "undoStatus": "pending", "x-vendor": 1});
        assert_eq!(set.created, Some(object));
        assert_eq!(set.not_created, None);
        let states = json!({"oldState": "1", "newState": "2"});
        assert_eq!(serde_json::Value::Object(set.rest), states);

        let set = raw::create(&mut client, "EmailSubmission", "e1", json!({})).unwrap();
        assert_eq!(set.created, None);
        assert_eq!(set.not_created, Some(json!({"type": "forbiddenFrom"})));
    }
}
