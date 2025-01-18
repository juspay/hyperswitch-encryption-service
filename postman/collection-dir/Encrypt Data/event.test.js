pm.test("Decrypt Data - Status code is 200", function () {
    pm.response.to.have.status(200);
});
var response = pm.response.json();

// Set the value of 'ff' to a variable named 'ff_value'
pm.environment.set("ff_value", response.data.ff);
