pm.test("Create Key - Status code is 200", function () {
    pm.response.to.have.status(200);
});

(function () {
    let jsonData = pm.response.json();
    pm.environment.set("key_version", jsonData.key_version);
    pm.environment.set("identifier", JSON.stringify(jsonData.identifier));
})();